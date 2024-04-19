use crate::state::{Mayland, State};
use libc::dev_t;
use smithay::{
	backend::{
		allocator::{
			dmabuf::Dmabuf, gbm::{GbmAllocator, GbmDevice}, Format as DrmFormat
		},
		drm::{compositor::DrmCompositor, DrmDevice, DrmDeviceFd, DrmEvent, DrmEventTime},
		egl::{EGLContext, EGLDisplay},
		input::InputEvent,
		libinput::{LibinputInputBackend, LibinputSessionInterface},
		renderer::{glow::GlowRenderer, Bind, ImportEgl},
		session::{libseat::LibSeatSession, Event as SessionEvent, Session},
		udev::{self, UdevBackend, UdevEvent},
	},
	desktop::utils::OutputPresentationFeedback,
	reexports::{
		calloop::{Dispatcher, RegistrationToken},
		drm::control::crtc,
		input::Libinput,
		rustix::fs::OFlags,
		wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
	},
	utils::{DeviceFd, Monotonic},
};
use smithay_drm_extras::drm_scanner::DrmScanner;
use std::{
	collections::{HashMap, HashSet},
	path::{Path, PathBuf},
	time::Duration,
};

type GbmDrmCompositor = DrmCompositor<
	GbmAllocator<DrmDeviceFd>,
	GbmDevice<DrmDeviceFd>,
	OutputPresentationFeedback,
	DrmDeviceFd,
>;

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
struct TtyOutputState {
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
			.insert_source(notifier, |event, _, state| match event {
				SessionEvent::ActivateSession => {
					println!("activate session");
				}
				SessionEvent::PauseSession => {
					println!("pause session");
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
	fn on_udev_data(&mut self, event: UdevEvent, mayland: &mut Mayland) {
		match event {
			UdevEvent::Added { device_id, path } => {
				if !self.session.is_active() {
					println!("session inactive");
					return;
				}

				self.device_added(device_id, &path, mayland);
			}
			UdevEvent::Changed { device_id } => {
				if !self.session.is_active() {
					println!("session inactive");
					return;
				}

				self.device_changed(device_id, mayland);
			}
			UdevEvent::Removed { device_id } => {
				if !self.session.is_active() {
					{
						println!("session inactive");
						return;
					}
				}

				self.device_removed(device_id, mayland);
			}
		}
	}

	fn device_added(&mut self, device_id: dev_t, path: &Path, mayland: &mut Mayland) {
		if path != self.primary_gpu_path {
			println!("skipping non-primary gpu");
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
								println!("error marking frame as submitted {}", err);
							}
						}

						// let output = state
						// 	.mayland
						// 	.space
						// 	.outputs()
						// 	.find(|output| {
						// 		let tty_state = output.user_data().get::<TtyOutputState>().unwrap();
						// 		tty_state.device_id == device.id && tty_state.crtc == crtc
						// 	})
						// 	.unwrap()
						// 	.clone();

						// let output_state = state.mayland.out
					}
					DrmEvent::Error(error) => println!("drm error {:?}", error),
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
		
	}

	fn device_removed(&mut self, device_id: dev_t, mayland: &mut Mayland) {}
}

impl State {
	fn handle_libinput_event(&mut self, event: &mut InputEvent<LibinputInputBackend>) {
		println!("libinput event");
	}
}
