use crate::{state::Mayland, State};
use smithay::{
	backend::{
		allocator::dmabuf::Dmabuf,
		renderer::{
			damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
			glow::GlowRenderer, ImportDma, ImportEgl,
		},
		winit::{self, WinitEvent, WinitGraphicsBackend},
	},
	output::{Mode, Output, PhysicalProperties, Subpixel},
	utils::{Rectangle, Transform},
};
use std::time::Duration;

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

		mayland.add_output(output.clone());

		let _global = output.create_global::<State>(&mayland.display_handle);
		output.change_current_state(
			Some(mode),
			Some(Transform::Flipped180),
			None,
			Some((0, 0).into()),
		);
		output.set_preferred(mode);

		if backend
			.renderer()
			.bind_wl_display(&mayland.display_handle)
			.is_ok()
		{
			println!("EGL hardware-acceleration enabled");
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
	pub fn render(&mut self, mayland: &mut Mayland) {
		let size = self.backend.window_size();
		let damage = Rectangle::from_loc_and_size((0, 0), size);

		self.backend.bind().unwrap();
		smithay::desktop::space::render_output::<_, WaylandSurfaceRenderElement<GlowRenderer>, _, _>(
			&self.output,
			self.backend.renderer(),
			1.0,
			0,
			[&mayland.space],
			&[],
			&mut self.damage_tracker,
			[0.1, 0.1, 0.1, 1.0],
		)
		.unwrap();
		self.backend.submit(Some(&[damage])).unwrap();

		mayland.space.elements().for_each(|window| {
			window.0.send_frame(
				&self.output,
				mayland.start_time.elapsed(),
				Some(Duration::ZERO),
				|_, _| Some(self.output.clone()),
			);
		});

		mayland.space.refresh();
		mayland.popups.cleanup();
		let _ = mayland.display_handle.flush_clients();

		// ask for redraw to schedule new frame.
		self.backend.window().request_redraw();
	}

	pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
		self.backend
			.renderer()
			.import_dmabuf(dmabuf, None)
			.inspect_err(|err| println!("error importing dmabuf: {:?}", err))
			.is_ok()
	}
}

impl State {
	fn handle_winit_event(&mut self, event: WinitEvent) {
		match event {
			focus @ WinitEvent::Focus(_) => println!("event {:?}", focus),
			WinitEvent::Resized { size, .. } => {
				let winit = self.backend.winit();
				let mode = Mode {
					size,
					refresh: 60_000,
				};

				if let Some(prev) = winit.output.current_mode() {
					winit.output.delete_mode(prev);
				}

				winit
					.output
					.change_current_state(Some(mode), None, None, None);
				winit.output.set_preferred(mode);

				winit.render(&mut self.mayland);
			}
			WinitEvent::Redraw => {
				self.mayland
					.queue_redraw(self.backend.winit().output.clone());
			}
			WinitEvent::CloseRequested => self.mayland.loop_signal.stop(),
			WinitEvent::Input(input) => self.handle_input_event(input),
		}
	}
}
