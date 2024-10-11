use super::BACKGROUND_COLOR;
use crate::{
	input::{apply_libinput_settings, device::InputDevice},
	state::{Mayland, MaylandRenderElements, State},
};
use libc::dev_t;
use smithay::{
	backend::{
		allocator::{
			dmabuf::Dmabuf,
			format::FormatSet,
			gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
			Fourcc,
		},
		drm::{compositor::DrmCompositor, DrmDevice, DrmDeviceFd, DrmEvent, DrmEventTime},
		egl::{EGLContext, EGLDevice, EGLDisplay},
		input::InputEvent,
		libinput::{LibinputInputBackend, LibinputSessionInterface},
		renderer::{glow::GlowRenderer, Bind, ImportDma, ImportEgl},
		session::{libseat::LibSeatSession, Event as SessionEvent, Session},
		udev::{self, UdevBackend, UdevEvent},
	},
	desktop::utils::OutputPresentationFeedback,
	output::{Mode, Output, OutputModeSource, PhysicalProperties, Subpixel},
	reexports::{
		calloop::{Dispatcher, RegistrationToken},
		drm::control::{connector, crtc, ModeTypeFlags},
		input::Libinput,
		rustix::fs::OFlags,
		wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
	},
	utils::{DeviceFd, Monotonic},
	wayland::dmabuf::{DmabufFeedbackBuilder, DmabufGlobal},
};
use smithay_drm_extras::{
	display_info,
	drm_scanner::{DrmScanEvent, DrmScanner},
};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	time::Duration,
};

type GbmDrmCompositor =
	DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmDevice<DrmDeviceFd>, OutputPresentationFeedback, DrmDeviceFd>;

const SUPPORTED_COLOR_FORMATS: &[Fourcc] = &[Fourcc::Argb8888, Fourcc::Abgr8888];

#[derive(Debug)]
pub struct OutputDevice {
	id: dev_t,
	token: RegistrationToken,
	drm: DrmDevice,
	gbm: GbmDevice<DrmDeviceFd>,
	glow: GlowRenderer,
	formats: FormatSet,
	drm_scanner: DrmScanner,
	surfaces: HashMap<crtc::Handle, GbmDrmCompositor>,
}

#[derive(Debug, Clone, Copy)]
struct UdevOutputState {
	device_id: dev_t,
	crtc: crtc::Handle,
}

#[derive(Debug)]
pub struct Udev {
	session: LibSeatSession,
	libinput: Libinput,

	udev_dispatcher: Dispatcher<'static, UdevBackend, State>,
	primary_gpu_path: PathBuf,
	output_device: Option<OutputDevice>,
	dmabuf_global: Option<DmabufGlobal>,
}

impl Udev {
	pub fn init(mayland: &mut Mayland) -> Self {
		let (session, notifier) = LibSeatSession::new().unwrap();
		let seat_name = session.seat();

		let udev_backend = UdevBackend::new(&seat_name).unwrap();
		let udev_dispatcher = Dispatcher::new(udev_backend, move |event, (), state: &mut State| {
			let udev = state.backend.udev();
			udev.on_udev_data(event, &mut state.mayland);
		});
		mayland
			.loop_handle
			.register_dispatcher(udev_dispatcher.clone())
			.unwrap();

		let libinput_session = LibinputSessionInterface::from(session.clone());
		let mut libinput = Libinput::new_with_udev(libinput_session);
		libinput.udev_assign_seat(&seat_name).unwrap();

		let input_backend = LibinputInputBackend::new(libinput.clone());
		mayland
			.loop_handle
			.insert_source(input_backend, |mut event, (), state| {
				state.handle_libinput_event(&mut event);
				state.handle_input_event(event);
			})
			.unwrap();

		mayland
			.loop_handle
			.insert_source(notifier, |event, (), state| {
				state.backend.udev().on_session_event(event, &mut state.mayland);
			})
			.unwrap();

		let primary_gpu_path = udev::primary_gpu(&seat_name).unwrap().unwrap();

		let mut udev = Udev {
			session,
			libinput,

			udev_dispatcher,
			primary_gpu_path,
			dmabuf_global: None,
			output_device: None,
		};

		for (device_id, path) in udev.udev_dispatcher.clone().as_source_ref().device_list() {
			udev.device_added(device_id, path, mayland);
		}

		udev
	}
}

impl Udev {
	pub fn render(&mut self, mayland: &mut Mayland, output: &Output, elements: &[MaylandRenderElements]) {
		let device = self.output_device.as_mut().unwrap();
		let tty_state: &UdevOutputState = output.user_data().get().unwrap();
		let drm_compositor = device.surfaces.get_mut(&tty_state.crtc).unwrap();

		match drm_compositor.render_frame(&mut device.glow, elements, BACKGROUND_COLOR) {
			Ok(render_output_res) => {
				mayland.post_repaint(output);

				let output_presentation_feedback =
					mayland.presentation_feedback(output, &render_output_res.states);

				if !render_output_res.is_empty {
					match drm_compositor.queue_frame(output_presentation_feedback) {
						Ok(()) => {
							let output_state = mayland.output_state.get_mut(output).unwrap();
							output_state.waiting_for_vblank = true;
						}
						Err(err) => tracing::error!("error queueing frame {:?}", err),
					}
				}
			}
			Err(err) => tracing::error!("error rendering frame {:?}", err),
		};
	}

	pub fn renderer(&mut self) -> &mut GlowRenderer {
		&mut self.output_device.as_mut().unwrap().glow
	}

	pub fn switch_vt(&mut self, vt: i32) {
		self.session.change_vt(vt).unwrap();
	}

	pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
		let Some(output_device) = self.output_device.as_mut() else {
			return false;
		};

		output_device
			.glow
			.import_dmabuf(dmabuf, None)
			.inspect_err(|err| tracing::error!("error importing dmabuf: {:?}", err))
			.is_ok()
	}
}

impl Udev {
	fn on_udev_data(&mut self, event: UdevEvent, mayland: &mut Mayland) {
		match event {
			UdevEvent::Added { device_id, path } => {
				if !self.session.is_active() {
					tracing::info!("session inactive");
					return;
				}

				self.device_added(device_id, &path, mayland);
			}
			UdevEvent::Changed { device_id } => {
				if !self.session.is_active() {
					tracing::info!("session inactive");
					return;
				}

				self.device_changed(device_id, mayland);
			}
			UdevEvent::Removed { device_id } => {
				if !self.session.is_active() {
					tracing::info!("session inactive");
					return;
				}

				self.device_removed(device_id, mayland);
			}
		}
	}

	fn on_session_event(&mut self, event: SessionEvent, mayland: &mut Mayland) {
		match event {
			SessionEvent::PauseSession => {
				tracing::info!("pause session");

				self.libinput.suspend();
				if let Some(device) = &mut self.output_device {
					device.drm.pause();
				}
			}
			SessionEvent::ActivateSession => {
				tracing::info!("activate session");

				self.libinput.resume().unwrap();
				if let Some(device) = &mut self.output_device {
					device.drm.activate(true).unwrap();
				}

				mayland.queue_redraw_all();
			}
		}
	}

	fn device_added(&mut self, device_id: dev_t, path: &Path, mayland: &mut Mayland) {
		if path != self.primary_gpu_path {
			tracing::info!("skipping non-primary gpu");
			return;
		}

		assert!(
			self.output_device.is_none(),
			"cannot add device if it already exists"
		);

		let flags = OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK;
		let fd = self.session.open(path, flags).unwrap();
		let fd = DrmDeviceFd::new(DeviceFd::from(fd));

		let (drm, drm_notifier) = DrmDevice::new(fd.clone(), true).unwrap();
		let gbm = GbmDevice::new(fd).unwrap();

		// SAFETY: this project doesn't use an egl display outside of smithay
		let display = unsafe { EGLDisplay::new(gbm.clone()) }.unwrap();
		let egl_context = EGLContext::new(&display).unwrap();

		// SAFETY: the egl context is only active in this thread
		let mut glow = unsafe { GlowRenderer::new(egl_context) }.unwrap();

		glow.bind_wl_display(&mayland.display_handle).unwrap();

		let egl_device = EGLDevice::device_for_display(glow.egl_context().display()).unwrap();
		let render_node = egl_device.try_get_render_node().unwrap().unwrap();

		let dmabuf_formats = glow.dmabuf_formats();
		let dmabuf_default_feedback = DmabufFeedbackBuilder::new(render_node.dev_id(), dmabuf_formats)
			.build()
			.unwrap();

		let dmabuf_global = mayland
			.dmabuf_state
			.create_global_with_default_feedback::<State>(&mayland.display_handle, &dmabuf_default_feedback);
		self.dmabuf_global = Some(dmabuf_global);

		let token = mayland
			.loop_handle
			.insert_source(drm_notifier, move |event, metadata, state| {
				let udev = state.backend.udev();
				match event {
					DrmEvent::VBlank(crtc) => {
						let device = udev.output_device.as_mut().unwrap();
						let drm_comp = device.surfaces.get_mut(&crtc).unwrap();

						let presentation_time = match metadata.as_mut().unwrap().time {
							DrmEventTime::Monotonic(time) => time,
							DrmEventTime::Realtime(_) => {
								// not supported
								Duration::ZERO
							}
						};

						match drm_comp.frame_submitted() {
							Ok(Some(mut feedback)) => {
								let seq = metadata.as_ref().map_or(0, |meta| meta.sequence);
								let flags = wp_presentation_feedback::Kind::Vsync
									| wp_presentation_feedback::Kind::HwClock
									| wp_presentation_feedback::Kind::HwCompletion;

								let output = feedback.output().unwrap();
								let refresh = output
									.current_mode()
									.map(|mode| Duration::from_secs_f64(1_000f64 / f64::from(mode.refresh)))
									.unwrap_or_default();

								feedback.presented::<_, Monotonic>(
									presentation_time,
									refresh,
									u64::from(seq),
									flags,
								);
							}
							Ok(None) => {}
							Err(err) => {
								tracing::error!("error marking frame as submitted {}", err);
							}
						}

						let output = state
							.mayland
							.workspaces
							.outputs()
							.find(|output| {
								let udev_state = output.user_data().get::<UdevOutputState>().unwrap();
								udev_state.device_id == device.id && udev_state.crtc == crtc
							})
							.unwrap()
							.clone();

						let output_state = state.mayland.output_state.get_mut(&output).unwrap();
						output_state.waiting_for_vblank = false;

						state.mayland.queue_redraw(output);
					}
					DrmEvent::Error(error) => tracing::error!("drm error {:?}", error),
				}
			})
			.unwrap();

		let formats = Bind::<Dmabuf>::supported_formats(&glow).unwrap_or_default();

		let output_device = OutputDevice {
			id: device_id,
			token,
			drm,
			gbm,
			glow,
			formats,
			drm_scanner: DrmScanner::new(),
			surfaces: HashMap::new(),
		};
		self.output_device = Some(output_device);

		self.device_changed(device_id, mayland);
	}

	fn device_changed(&mut self, device_id: dev_t, mayland: &mut Mayland) {
		let Some(device) = &mut self.output_device else { return };
		if device.id != device_id {
			return;
		}

		let scan_result = match device.drm_scanner.scan_connectors(&device.drm) {
			Ok(scan_result) => scan_result,
			Err(err) => {
				tracing::warn!(?err, "failed to scan connector");
				return;
			}
		};

		for event in scan_result {
			match event {
				DrmScanEvent::Connected {
					connector,
					crtc: Some(crtc),
				} => {
					self.connector_connected(connector, crtc, mayland);
				}
				DrmScanEvent::Disconnected {
					connector,
					crtc: Some(crtc),
				} => {
					self.connector_disconnected(connector, crtc, mayland);
				}
				_ => {}
			}
		}
	}

	fn device_removed(&mut self, device_id: dev_t, mayland: &mut Mayland) {
		let Some(device) = &mut self.output_device else { return };
		if device_id != device.id {
			return;
		}

		let crtcs = device
			.drm_scanner
			.crtcs()
			.map(|(info, crtc)| (info.clone(), crtc))
			.collect::<Vec<_>>();
		for (connector, crtc) in crtcs {
			self.connector_disconnected(connector, crtc, mayland);
		}

		let mut device = self.output_device.take().unwrap();
		device.glow.unbind_wl_display();

		// todo disable first
		let dmabuf_global = self.dmabuf_global.take().unwrap();
		mayland
			.dmabuf_state
			.destroy_global::<State>(&mayland.display_handle, dmabuf_global);

		mayland.loop_handle.remove(device.token);
	}

	fn connector_connected(&mut self, connector: connector::Info, crtc: crtc::Handle, mayland: &mut Mayland) {
		let output_name = format!("{}-{}", connector.interface().as_str(), connector.interface_id());
		tracing::info!("connecting connector: {}", output_name);

		let mode = connector
			.modes()
			.iter()
			.filter(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
			.max_by_key(|m| m.vrefresh())
			.unwrap();

		let device = self.output_device.as_mut().unwrap();
		let surface = device
			.drm
			.create_surface(crtc, *mode, &[connector.handle()])
			.unwrap();

		let mut planes = surface.planes().clone();

		// overlay planes need to be cleared when switching vt to
		// avoid the windows getting stuck on the monitor when switching
		// to a compositor that doesn't clean overlay planes on activate
		// todo find a better way to do this
		planes.overlay.clear();

		let gbm_flags = GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT;
		let allocator = GbmAllocator::new(device.gbm.clone(), gbm_flags);

		let (physical_width, physical_height) = connector.size().unwrap_or((0, 0));

		let info = display_info::for_connector(&device.drm, connector.handle());
		let make = info
			.as_ref()
			.and_then(|info| info.make())
			.unwrap_or_else(|| "unknown".to_owned());
		let model = info
			.as_ref()
			.and_then(|info| info.model())
			.unwrap_or_else(|| "unknown".to_owned());

		let output = Output::new(
			output_name,
			PhysicalProperties {
				size: (physical_width as i32, physical_height as i32).into(),
				subpixel: Subpixel::Unknown,
				model,
				make,
			},
		);

		let wl_mode = Mode::from(*mode);
		output.change_current_state(Some(wl_mode), None, None, None);
		output.set_preferred(wl_mode);

		output.user_data().insert_if_missing(|| UdevOutputState {
			device_id: device.id,
			crtc,
		});

		let compositor = DrmCompositor::new(
			OutputModeSource::Auto(output.clone()),
			surface,
			Some(planes),
			allocator,
			device.gbm.clone(),
			SUPPORTED_COLOR_FORMATS,
			device.formats.clone(),
			device.drm.cursor_size(),
			Some(device.gbm.clone()),
		)
		.unwrap();

		let res = device.surfaces.insert(crtc, compositor);
		assert!(res.is_none(), "crtc must not have already existed");

		mayland.add_output(output.clone());
		mayland.queue_redraw(output);
	}

	fn connector_disconnected(
		&mut self,
		connector: connector::Info,
		crtc: crtc::Handle,
		mayland: &mut Mayland,
	) {
		tracing::info!("disconnecting connector {:?}", connector);
		let device = self.output_device.as_mut().unwrap();

		if device.surfaces.remove(&crtc).is_none() {
			tracing::info!("crtc wasn't enabled");
			return;
		}

		let output = mayland
			.workspaces
			.outputs()
			.find(|output| {
				let udev_state = output.user_data().get::<UdevOutputState>().unwrap();
				udev_state.crtc == crtc && udev_state.device_id == device.id
			})
			.unwrap()
			.clone();

		mayland.remove_output(&output);
	}
}

impl State {
	fn handle_libinput_event(&mut self, event: &mut InputEvent<LibinputInputBackend>) {
		if let InputEvent::DeviceAdded { device } = event {
			let devices = InputDevice::new(device);
			for mut device in devices {
				let config = &self.mayland.config;
				apply_libinput_settings(&config.input, &mut device);
				self.mayland.devices.insert(device);
			}
		}
	}
}
