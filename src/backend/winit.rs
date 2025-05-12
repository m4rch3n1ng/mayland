use crate::{
	render::{MaylandRenderElements, shaders},
	state::{Mayland, State},
	utils::logical_output,
};
use mayland_config::outputs::OutputInfo;
use smithay::{
	backend::{
		allocator::dmabuf::Dmabuf,
		egl::EGLDevice,
		renderer::{ImportDma, ImportEgl, damage::OutputDamageTracker, glow::GlowRenderer},
		winit::{self, WinitEvent, WinitGraphicsBackend},
	},
	output::{Mode, Output, PhysicalProperties, Subpixel},
	reexports::winit::window::Window,
	utils::Transform,
	wayland::dmabuf::DmabufFeedbackBuilder,
};
use std::borrow::BorrowMut;

#[derive(Debug)]
pub struct Winit {
	backend: WinitGraphicsBackend<GlowRenderer>,
	output: Output,
	damage_tracker: OutputDamageTracker,
}

impl Winit {
	pub fn init(mayland: &mut Mayland) -> Self {
		let window = Window::default_attributes().with_title("mayland");
		let (mut backend, winit_evt) = winit::init_from_attributes::<GlowRenderer>(window).unwrap();
		backend.window().set_cursor_visible(false);

		shaders::init(backend.renderer().borrow_mut());

		let mode = Mode {
			size: backend.window_size(),
			refresh: 60_000,
		};

		let output = Output::new(
			"winit".to_owned(),
			PhysicalProperties {
				size: (0, 0).into(),
				subpixel: Subpixel::Unknown,
				make: "may".to_owned(),
				model: "winit".to_owned(),
			},
		);

		let _global = output.create_global::<State>(&mayland.display_handle);
		output.change_current_state(Some(mode), Some(Transform::Flipped180), None, None);
		output.set_preferred(mode);

		output.user_data().insert_if_missing(|| OutputInfo {
			connector: "winit".to_owned(),
			make: "may".to_owned(),
			model: "winit".to_owned(),
			serial: None,
		});

		mayland.add_output(output.clone());

		let egl_device = EGLDevice::device_for_display(backend.renderer().egl_context().display()).unwrap();
		let render_node = egl_device.try_get_render_node().unwrap();

		if let Some(node) = render_node {
			let dmabuf_formats = backend.renderer().dmabuf_formats();
			let dmabuf_default_feedback = DmabufFeedbackBuilder::new(node.dev_id(), dmabuf_formats)
				.build()
				.unwrap();

			mayland.dmabuf_state.create_global_with_default_feedback::<State>(
				&mayland.display_handle,
				&dmabuf_default_feedback,
			);
		} else {
			tracing::warn!("no render node");
		}

		if backend
			.renderer()
			.bind_wl_display(&mayland.display_handle)
			.is_ok()
		{
			tracing::info!("EGL hardware-acceleration enabled");
		}

		let damage_tracker = OutputDamageTracker::from_output(&output);
		let winit = Winit {
			backend,
			output,
			damage_tracker,
		};

		mayland
			.loop_handle
			.insert_source(winit_evt, |event, (), state| {
				state.handle_winit_event(event);
			})
			.unwrap();

		winit
	}
}

impl Winit {
	pub fn render(&mut self, mayland: &mut Mayland, output: &Output, elements: &[MaylandRenderElements]) {
		let result = {
			let (renderer, mut fb) = self.backend.bind().unwrap();
			self.damage_tracker
				.render_output(renderer, &mut fb, 0, elements, [0.; 4])
				.unwrap()
		};

		if let Some(damage) = result.damage {
			self.backend.submit(Some(damage)).unwrap();

			mayland.send_frame_callbacks(output);

			// ask for redraw to schedule new frame.
			self.backend.window().request_redraw();
		}
	}

	pub fn renderer(&mut self) -> &mut GlowRenderer {
		self.backend.renderer()
	}

	pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
		self.backend
			.renderer()
			.import_dmabuf(dmabuf, None)
			.inspect_err(|err| tracing::error!("error importing dmabuf: {:?}", err))
			.is_ok()
	}

	pub fn comm_outputs(&self) -> Vec<mayland_comm::Output> {
		let mode = mayland_comm::output::Mode {
			w: self.backend.window_size().w.clamp(0, u16::MAX as i32) as u16,
			h: self.backend.window_size().h.clamp(0, u16::MAX as i32) as u16,
			refresh: 60_000,

			preferred: true,
		};

		let physical = self.output.physical_properties();
		let logical = logical_output(&self.output);

		let output = mayland_comm::Output {
			name: self.output.name(),
			mode: Some(mode),
			make: physical.make,
			model: physical.model,
			serial: None,
			size: None,
			logical: Some(logical),
			modes: vec![mode],
		};
		vec![output]
	}

	pub fn reload_output_config(&mut self, mayland: &mut Mayland) {
		mayland.reconfigure_outputs();
		mayland.queue_redraw(self.output.clone());
	}
}

impl State {
	fn handle_winit_event(&mut self, event: WinitEvent) {
		match event {
			WinitEvent::Focus(_) => (),
			WinitEvent::Resized { size, .. } => {
				let winit = self.backend.winit();
				let mode = Mode {
					size,
					refresh: 60_000,
				};

				if let Some(prev) = winit.output.current_mode() {
					winit.output.delete_mode(prev);
				}

				winit.output.change_current_state(Some(mode), None, None, None);
				winit.output.set_preferred(mode);

				self.mayland.output_size_changed(&winit.output);
				self.mayland.reconfigure_outputs();
				self.mayland.queue_redraw(winit.output.clone());
			}
			WinitEvent::Redraw => {
				self.mayland.queue_redraw(self.backend.winit().output.clone());
			}
			WinitEvent::CloseRequested => self.mayland.loop_signal.stop(),
			WinitEvent::Input(input) => self.handle_input_event(input),
		}
	}
}
