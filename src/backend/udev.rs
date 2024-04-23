use super::BACKGROUND_COLOR;
use crate::{
	render::MaylandRenderElements,
	state::{Mayland, State},
};
use libc::dev_t;
use smithay::{
	backend::{
		allocator::{
			dmabuf::Dmabuf,
			gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
			Format as DrmFormat, Fourcc,
		},
		drm::{compositor::DrmCompositor, DrmDevice, DrmDeviceFd, DrmEvent, DrmEventTime},
		egl::{EGLContext, EGLDisplay},
		input::InputEvent,
		libinput::{LibinputInputBackend, LibinputSessionInterface},
		renderer::{
			element::surface::WaylandSurfaceRenderElement, glow::GlowRenderer, Bind, ImportDma,
			ImportEgl,
		},
		session::{libseat::LibSeatSession, Event as SessionEvent, Session},
		udev::{self, UdevBackend, UdevEvent},
	},
	desktop::utils::OutputPresentationFeedback,
	output::{Mode, Output, OutputModeSource, PhysicalProperties, Subpixel},
	reexports::{
		calloop::{Dispatcher, RegistrationToken},
		drm::control::{connector, crtc, ModeTypeFlags},
		input::{AccelProfile, Libinput},
		rustix::fs::OFlags,
		wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
	},
	utils::{DeviceFd, Monotonic},
};
use smithay_drm_extras::{
	drm_scanner::{DrmScanEvent, DrmScanner},
	edid::EdidInfo,
};
use std::{
	collections::{HashMap, HashSet},
	path::{Path, PathBuf},
	time::Duration,
};
use tracing::{error, info};

type GbmDrmCompositor = DrmCompositor<
	GbmAllocator<DrmDeviceFd>,
	GbmDevice<DrmDeviceFd>,
	OutputPresentationFeedback,
	DrmDeviceFd,
>;

const SUPPORTED_COLOR_FORMATS: &[Fourcc] = &[Fourcc::Argb8888, Fourcc::Abgr8888];

#[derive(Debug)]
pub struct OutputDevice {
	id: dev_t,
	token: RegistrationToken,
	drm: DrmDevice,
	gbm: GbmDevice<DrmDeviceFd>,
	glow: GlowRenderer,
	formats: HashSet<DrmFormat>,
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
	udev_dispatcher: Dispatcher<'static, UdevBackend, State>,
	primary_gpu_path: PathBuf,
	output_device: Option<OutputDevice>,
}

impl Udev {
	pub fn init(mayland: &mut Mayland) -> Self {
		let (session, notifier) = LibSeatSession::new().unwrap();
		let seat_name = session.seat();

		let udev_backend = UdevBackend::new(&seat_name).unwrap();
		let udev_dispatcher = Dispatcher::new(udev_backend, move |event, _, state: &mut State| {
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
			.insert_source(input_backend, |mut event, _, state| {
				state.handle_libinput_event(&mut event);
				state.handle_input_event(event);
			})
			.unwrap();

		mayland
			.loop_handle
			.insert_source(notifier, |event, _, _state| match event {
				SessionEvent::ActivateSession => {
					info!("activate session");
				}
				SessionEvent::PauseSession => {
					info!("pause session");
				}
			})
			.unwrap();

		let primary_gpu_path = udev::primary_gpu(&seat_name).unwrap().unwrap();

		let mut udev = Udev {
			session,
			udev_dispatcher,
			primary_gpu_path,
			output_device: None,
		};

		for (device_id, path) in udev.udev_dispatcher.clone().as_source_ref().device_list() {
			udev.device_added(device_id, path, mayland);
		}

		udev
	}
}

impl Udev {
	pub fn render(
		&mut self,
		mayland: &mut Mayland,
		output: &Output,
		elements: &[MaylandRenderElements<
			GlowRenderer,
			WaylandSurfaceRenderElement<GlowRenderer>,
		>],
	) {
		let device = self.output_device.as_mut().unwrap();
		let tty_state: &UdevOutputState = output.user_data().get().unwrap();
		let drm_compositor = device.surfaces.get_mut(&tty_state.crtc).unwrap();

		match drm_compositor.render_frame(&mut device.glow, elements, BACKGROUND_COLOR) {
			Ok(render_output_res) => {
				mayland.post_repaint(output);

				let output_presentation_feedback =
					mayland.presentation_feedback(output, &render_output_res.states);

				match drm_compositor.queue_frame(output_presentation_feedback) {
					Ok(()) => {
						let output_state = mayland.output_state.get_mut(output).unwrap();
						output_state.waiting_for_vblank = true;
					}
					Err(err) => error!("error queueing frame {:?}", err),
				}
			}
			Err(err) => error!("error rendering frame {:?}", err),
		};
	}

	pub fn renderer(&mut self) -> &mut GlowRenderer {
		&mut self.output_device.as_mut().unwrap().glow
	}

	pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
		let Some(output_device) = self.output_device.as_mut() else {
			return false;
		};

		output_device
			.glow
			.import_dmabuf(dmabuf, None)
			.inspect_err(|err| error!("error importing dmabuf: {:?}", err))
			.is_ok()
	}
}

impl Udev {
	fn on_udev_data(&mut self, event: UdevEvent, mayland: &mut Mayland) {
		match event {
			UdevEvent::Added { device_id, path } => {
				if !self.session.is_active() {
					info!("session inactive");
					return;
				}

				self.device_added(device_id, &path, mayland);
			}
			UdevEvent::Changed { device_id } => {
				if !self.session.is_active() {
					info!("session inactive");
					return;
				}

				self.device_changed(device_id, mayland);
			}
			UdevEvent::Removed { device_id } => {
				if !self.session.is_active() {
					info!("session inactive");
					return;
				}

				self.device_removed(device_id, mayland);
			}
		}
	}

	fn device_added(&mut self, device_id: dev_t, path: &Path, mayland: &mut Mayland) {
		if path != self.primary_gpu_path {
			info!("skipping non-primary gpu");
			return;
		}

		assert!(self.output_device.is_none());

		let flags = OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK;
		let fd = self.session.open(path, flags).unwrap();
		let fd = DrmDeviceFd::new(DeviceFd::from(fd));

		let (drm, drm_notifier) = DrmDevice::new(fd.clone(), true).unwrap();
		let gbm = GbmDevice::new(fd).unwrap();

		let display = unsafe { EGLDisplay::new(gbm.clone()) }.unwrap();
		let egl_context = EGLContext::new(&display).unwrap();

		let mut glow = unsafe { GlowRenderer::new(egl_context) }.unwrap();
		glow.bind_wl_display(&mayland.display_handle).unwrap();

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
								let seq = metadata.as_ref().map(|meta| meta.sequence).unwrap_or(0);
								let flags = wp_presentation_feedback::Kind::Vsync
									| wp_presentation_feedback::Kind::HwClock
									| wp_presentation_feedback::Kind::HwCompletion;

								let output = feedback.output().unwrap();
								let refresh = output
									.current_mode()
									.map(|mode| {
										Duration::from_secs_f64(1_000f64 / mode.refresh as f64)
									})
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
								error!("error marking frame as submitted {}", err);
							}
						}

						let output = state
							.mayland
							.space
							.outputs()
							.find(|output| {
								let tty_state =
									output.user_data().get::<UdevOutputState>().unwrap();
								tty_state.device_id == device.id && tty_state.crtc == crtc
							})
							.unwrap()
							.clone();

						let output_state = state.mayland.output_state.get_mut(&output).unwrap();
						output_state.waiting_for_vblank = false;

						state.mayland.queue_redraw(output);
					}
					DrmEvent::Error(error) => error!("drm error {:?}", error),
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
		let Some(device) = &mut self.output_device else {
			return;
		};
		if device.id != device_id {
			return;
		}

		for event in device.drm_scanner.scan_connectors(&device.drm) {
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
		let Some(device) = &mut self.output_device else {
			return;
		};
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
		mayland.loop_handle.remove(device.token);
	}

	fn connector_connected(
		&mut self,
		connector: connector::Info,
		crtc: crtc::Handle,
		mayland: &mut Mayland,
	) {
		let output_name = format!(
			"{}-{}",
			connector.interface().as_str(),
			connector.interface_id(),
		);
		info!("connecting connector: {output_name}");

		let device = self.output_device.as_mut().unwrap();

		let mode = connector
			.modes()
			.iter()
			.filter(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
			.max_by_key(|m| m.vrefresh())
			.unwrap();

		let surface = device
			.drm
			.create_surface(crtc, *mode, &[connector.handle()])
			.unwrap();

		let gbm_flags = GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT;
		let allocator = GbmAllocator::new(device.gbm.clone(), gbm_flags);

		let (physical_width, physical_height) = connector.size().unwrap_or((0, 0));

		let (make, model) = EdidInfo::for_connector(&device.drm, connector.handle())
			.map(|info| (info.manufacturer, info.model))
			.unwrap_or_else(|| ("Unknown".into(), "Unknown".into()));

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
		output.change_current_state(Some(wl_mode), None, None, Some((0, 0).into()));
		output.set_preferred(wl_mode);

		output.user_data().insert_if_missing(|| UdevOutputState {
			device_id: device.id,
			crtc,
		});

		let compositor = DrmCompositor::new(
			OutputModeSource::Auto(output.clone()),
			surface,
			None,
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
		info!("disconnecting connector {:?}", connector);
		let device = self.output_device.as_mut().unwrap();

		if device.surfaces.remove(&crtc).is_none() {
			info!("crtc wasn't enabled");
			return;
		}

		let output = mayland
			.space
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
			let is_touchpad = device.config_tap_finger_count() > 0;
			if is_touchpad {
				let _ = device.config_tap_set_enabled(true);
				let _ = device.config_accel_set_profile(AccelProfile::Flat);
				let _ = device.config_scroll_set_natural_scroll_enabled(true);
			}
		}
	}
}
