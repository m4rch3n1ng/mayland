use super::BACKGROUND_COLOR;
use crate::{render::MaylandRenderElements, state::Mayland, State};
use smithay::{
	backend::{
		allocator::dmabuf::Dmabuf,
		renderer::{damage::OutputDamageTracker, glow::GlowRenderer, ImportDma, ImportEgl},
		winit::{self, WinitEvent, WinitGraphicsBackend},
	},
	output::{Mode, Output, PhysicalProperties, Subpixel},
	utils::{Rectangle, Transform},
};
use tracing::{error, info};

#[derive(Debug)]
pub struct Winit {
	backend: WinitGraphicsBackend<GlowRenderer>,
	output: Output,
	damage_tracker: OutputDamageTracker,
}

impl Winit {
	pub fn init(mayland: &mut Mayland) -> Self {
		let (mut backend, winit_evt) = winit::init::<GlowRenderer>().unwrap();

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

		mayland.add_output(output.clone());

		if backend
			.renderer()
			.bind_wl_display(&mayland.display_handle)
			.is_ok()
		{
			info!("EGL hardware-acceleration enabled");
		};

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
		let size = self.backend.window_size();
		let damage = Rectangle::from_loc_and_size((0, 0), size);

		self.backend.bind().unwrap();
		let renderer = self.backend.renderer();
		self.damage_tracker
			.render_output(renderer, 0, elements, BACKGROUND_COLOR)
			.unwrap();

		self.backend.submit(Some(&[damage])).unwrap();

		mayland.post_repaint(output);

		// ask for redraw to schedule new frame.
		self.backend.window().request_redraw();
	}

	pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
		self.backend
			.renderer()
			.import_dmabuf(dmabuf, None)
			.inspect_err(|err| error!("error importing dmabuf: {:?}", err))
			.is_ok()
	}

	pub fn renderer(&mut self) -> &mut GlowRenderer {
		self.backend.renderer()
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

				self.mayland.output_resized(&winit.output);
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
