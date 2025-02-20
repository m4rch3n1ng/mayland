use crate::{
	input::{apply_libinput_settings, device::InputDevice},
	render::{MaylandRenderElements, shaders},
	state::{Mayland, State},
	utils::logical_output,
};
use libc::dev_t;
use mayland_config::outputs::OutputInfo;
use smithay::{
	backend::{
		allocator::{
			Fourcc,
			dmabuf::Dmabuf,
			format::FormatSet,
			gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
		},
		drm::{
			DrmDevice, DrmDeviceFd, DrmEvent, DrmEventMetadata, DrmEventTime,
			compositor::{DrmCompositor, FrameFlags},
		},
		egl::{EGLContext, EGLDevice, EGLDisplay},
		input::InputEvent,
		libinput::{LibinputInputBackend, LibinputSessionInterface},
		renderer::{ImportDma, ImportEgl, glow::GlowRenderer},
		session::{Event as SessionEvent, Session, libseat::LibSeatSession},
		udev::{self, UdevBackend, UdevEvent},
	},
	desktop::utils::OutputPresentationFeedback,
	output::{Mode, Output, OutputModeSource, PhysicalProperties, Subpixel},
	reexports::{
		calloop::{Dispatcher, RegistrationToken},
		drm::control::{self, ModeFlags, ModeTypeFlags, connector, crtc},
		gbm::Modifier,
		input::Libinput,
		rustix::fs::OFlags,
		wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
	},
	utils::{DeviceFd, Monotonic},
	wayland::{
		dmabuf::{DmabufFeedbackBuilder, DmabufGlobal},
		presentation::Refresh,
	},
};
use smithay_drm_extras::{
	display_info,
	drm_scanner::{DrmScanEvent, DrmScanner},
};
use std::{
	borrow::BorrowMut,
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
pub struct UdevOutputState {
	pub device_id: dev_t,
	pub crtc: crtc::Handle,
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
		let udev_state = output.user_data().get::<UdevOutputState>().unwrap();
		let drm_compositor = device.surfaces.get_mut(&udev_state.crtc).unwrap();

		match drm_compositor.render_frame(&mut device.glow, elements, [0.; 4], FrameFlags::DEFAULT) {
			Ok(render_output_res) => {
				if render_output_res.is_empty {
					return;
				}

				let output_presentation_feedback =
					mayland.presentation_feedback(output, &render_output_res.states);

				match drm_compositor.queue_frame(output_presentation_feedback) {
					Ok(()) => {
						let output_state = mayland.output_state.get_mut(output).unwrap();
						output_state.queued.waiting_for_vblank();
					}
					Err(err) => tracing::error!("error queueing frame {:?}", err),
				}
			}
			Err(err) => {
				drm_compositor.reset_buffers();
				tracing::error!("error rendering frame {:?}", err);
			}
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

	pub fn comm_outputs(&self, mayland: &Mayland) -> Vec<mayland_comm::Output> {
		let mut outputs = Vec::new();

		if let Some(device) = &self.output_device {
			for (connector, crtc) in device.drm_scanner.crtcs() {
				let surface = device.surfaces.get(&crtc);
				let mode = surface.map(|surface| surface.pending_mode());
				let mode = mode.map(|mode| mayland_comm::output::Mode {
					w: mode.size().0,
					h: mode.size().1,
					refresh: Mode::from(mode).refresh as u32,

					preferred: mode.mode_type().contains(ModeTypeFlags::PREFERRED),
				});

				let modes = connector
					.modes()
					.iter()
					.map(|mode| {
						let wl_mode = Mode::from(*mode);
						mayland_comm::output::Mode {
							w: mode.size().0,
							h: mode.size().1,
							refresh: u32::try_from(wl_mode.refresh).unwrap(),

							preferred: mode.mode_type().contains(ModeTypeFlags::PREFERRED),
						}
					})
					.collect();

				let logical = mayland
					.workspaces
					.udev_output(device.id, crtc)
					.map(logical_output);

				let output_info = output_info(&device.drm, connector);
				let size = connector.size();

				let output = mayland_comm::Output {
					name: output_info.connector,
					mode,
					make: output_info.make,
					model: output_info.model,
					serial: output_info.serial,
					size,
					logical,
					modes,
				};
				outputs.push(output);
			}
		}

		outputs
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
		shaders::init(glow.borrow_mut());

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
						let metadata = metadata.expect("vblank events must have metadata");
						udev.on_vblank(&mut state.mayland, crtc, metadata);
					}
					DrmEvent::Error(error) => tracing::error!("drm error {:?}", error),
				}
			})
			.unwrap();

		let formats = glow.egl_context().dmabuf_render_formats().clone();

		// when upgrading to mesa 23.3.0, creating the DrmCompositor fails
		// with a "failed to add framebuffer, invalid argument" error
		//
		// filtering out the ccs modifiers seems to fix the issue
		//
		// taken from niri (SPDX: GPL-3.0-or-later)
		// https://github.com/YaLTeR/niri/blob/9c7e8d0/src/backend/tty.rs#L928-L956
		let formats = formats
			.iter()
			.copied()
			.filter(|format| {
				!matches!(
					format.modifier,
					Modifier::I915_y_tiled_ccs
					| Modifier::Unrecognized(0x100000000000005)
					| Modifier::I915_y_tiled_gen12_rc_ccs
					| Modifier::I915_y_tiled_gen12_mc_ccs
					// I915_FORMAT_MOD_Y_TILED_GEN12_RC_CCS_CC
					| Modifier::Unrecognized(0x100000000000008)
					// I915_FORMAT_MOD_4_TILED_DG2_RC_CCS
					| Modifier::Unrecognized(0x10000000000000a)
					// I915_FORMAT_MOD_4_TILED_DG2_MC_CCS
					| Modifier::Unrecognized(0x10000000000000b)
					// I915_FORMAT_MOD_4_TILED_DG2_RC_CCS_CC
					| Modifier::Unrecognized(0x10000000000000c)
				)
			})
			.collect::<FormatSet>();

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
		let device = self.output_device.as_mut().unwrap();

		let output_info = output_info(&device.drm, &connector);
		tracing::info!("connecting connector: {:?}", output_info);

		let config = mayland.config.output.get_output(&output_info);
		let mode = pick_mode(&connector, config.and_then(|conf| conf.mode));

		let surface = device
			.drm
			.create_surface(crtc, mode, &[connector.handle()])
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

		let output = Output::new(
			output_info.connector.clone(),
			PhysicalProperties {
				size: (physical_width as i32, physical_height as i32).into(),
				subpixel: Subpixel::Unknown,
				make: output_info.make.clone(),
				model: output_info.model.clone(),
			},
		);

		let wl_mode = Mode::from(mode);
		output.change_current_state(Some(wl_mode), None, None, None);
		output.set_preferred(wl_mode);

		output.user_data().insert_if_missing(|| output_info);
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
			SUPPORTED_COLOR_FORMATS.iter().copied(),
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

		let output = mayland.workspaces.udev_output(device.id, crtc).unwrap().clone();
		mayland.remove_output(&output);
	}

	fn on_vblank(&mut self, mayland: &mut Mayland, crtc: crtc::Handle, meta: DrmEventMetadata) {
		let device = self.output_device.as_mut().unwrap();
		let Some(surface) = device.surfaces.get_mut(&crtc) else {
			tracing::warn!("missing crtc {:?} in vblannk callback", crtc);
			return;
		};

		let presentation_time = match meta.time {
			DrmEventTime::Monotonic(time) => time,
			DrmEventTime::Realtime(_) => {
				// not supported
				Duration::ZERO
			}
		};

		match surface.frame_submitted() {
			Ok(Some(mut feedback)) => {
				let seq = meta.sequence;
				let flags = wp_presentation_feedback::Kind::Vsync
					| wp_presentation_feedback::Kind::HwClock
					| wp_presentation_feedback::Kind::HwCompletion;

				let output = feedback.output().unwrap();
				let refresh = output
					.current_mode()
					.map(|mode| Duration::from_secs_f64(1_000f64 / f64::from(mode.refresh)))
					.map(Refresh::Fixed)
					.unwrap_or(Refresh::Unknown);

				feedback.presented::<_, Monotonic>(presentation_time, refresh, u64::from(seq), flags);
			}
			Ok(None) => {}
			Err(err) => {
				tracing::error!("error marking frame as submitted {}", err);
			}
		}

		let output = mayland.workspaces.udev_output(device.id, crtc).unwrap().clone();

		let output_state = mayland.output_state.get_mut(&output).unwrap();
		output_state.queued.on_vblank();

		mayland.send_frame_callbacks(&output);
	}
}

fn output_info(drm: &DrmDevice, connector: &connector::Info) -> OutputInfo {
	let info = display_info::for_connector(drm, connector.handle());

	let connector = format!("{}-{}", connector.interface().as_str(), connector.interface_id());
	let make = info
		.as_ref()
		.and_then(|info| info.make())
		.unwrap_or_else(|| "unknown".to_owned());
	let model = info
		.as_ref()
		.and_then(|info| info.model())
		.unwrap_or_else(|| "unknown".to_owned());
	let serial = info.as_ref().and_then(|info| info.serial());

	OutputInfo {
		connector,
		make,
		model,
		serial,
	}
}

fn pick_mode(connector: &connector::Info, target: Option<mayland_config::outputs::Mode>) -> control::Mode {
	// try to match a mode from config
	if let Some(target) = target {
		let modes = connector
			.modes()
			.iter()
			.filter(|m| m.size() == (target.width, target.height))
			.filter(|m| !m.flags().contains(ModeFlags::INTERLACE));

		let mode = if let Some(refresh) = target.refresh {
			// get the one with the closest refresh rates
			modes.min_by_key(|m| i32::abs_diff(Mode::from(**m).refresh, refresh))
		} else {
			// get the one with the highest refresh rate
			modes.max_by_key(|m| m.vrefresh())
		};

		if let Some(mode) = mode {
			return *mode;
		} else {
			tracing::warn!("couldn't find matching mode");
		}
	}

	// try to get a preferred mode
	if let Some(mode) = connector
		.modes()
		.iter()
		.filter(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
		.max_by_key(|m| (m.size(), m.vrefresh()))
	{
		return *mode;
	}

	// pick the highest quality one that's not interlaced
	if let Some(mode) = connector
		.modes()
		.iter()
		.filter(|mode| !mode.flags().contains(ModeFlags::INTERLACE))
		.max_by_key(|m| (m.size(), m.vrefresh()))
	{
		return *mode;
	}

	// just pick the highest quality one
	if let Some(mode) = connector.modes().iter().max_by_key(|m| (m.size(), m.vrefresh())) {
		return *mode;
	}

	// what
	panic!("no modes available for this output?");
}

impl State {
	fn handle_libinput_event(&mut self, event: &mut InputEvent<LibinputInputBackend>) {
		match event {
			InputEvent::DeviceAdded { device } => {
				let devices = InputDevice::split(device);
				for mut device in devices {
					let config = &self.mayland.config;

					apply_libinput_settings(&config.input, &mut device);
					self.mayland.devices.insert(device);
				}
			}
			InputEvent::DeviceRemoved { device } => {
				self.mayland.devices.retain(|dev| &dev.handle != device);
			}
			_ => (),
		}
	}
}
