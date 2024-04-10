use crate::MayState;
use smithay::{
	backend::{
		renderer::{
			damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
			glow::GlowRenderer,
		},
		winit::{self, WinitEvent, WinitGraphicsBackend},
	},
	output::{Mode, Output, PhysicalProperties, Subpixel},
	reexports::calloop::EventLoop,
	utils::{Rectangle, Transform},
};
use std::time::Duration;

struct WinitData {
	backend: WinitGraphicsBackend<GlowRenderer>,
	output: Output,
	damage_tracker: OutputDamageTracker,
}

pub fn init(calloop: &mut EventLoop<MayState>, state: &mut MayState) {
	let display_handle = &mut state.display_handle;

	let (backend, winit) = winit::init::<GlowRenderer>().unwrap();

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

	state.space.map_output(&output, (0, 0));

	let _global = output.create_global::<MayState>(display_handle);
	output.change_current_state(
		Some(mode),
		Some(Transform::Flipped180),
		None,
		Some((0, 0).into()),
	);
	output.set_preferred(mode);

	let damage_tracker = OutputDamageTracker::from_output(&output);
	let mut winit_data = WinitData {
		backend,
		output,
		damage_tracker,
	};

	std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);
	std::env::set_var("GDK_BACKEND", "wayland");

	calloop
		.handle()
		.insert_source(winit, move |event, (), state| {
			state.handle_winit_event(event, &mut winit_data);
		})
		.unwrap();
}

impl WinitData {
	fn render(&mut self, state: &mut MayState) {
		let size = self.backend.window_size();
		let damage = Rectangle::from_loc_and_size((0, 0), size);

		self.backend.bind().unwrap();
		smithay::desktop::space::render_output::<_, WaylandSurfaceRenderElement<GlowRenderer>, _, _>(
			&self.output,self.backend.renderer(),
			1.0,
			0,
			[&state.space],
			&[],
			&mut self.damage_tracker,
			[0.1, 0.1, 0.1, 1.0]
		).unwrap();
		self.backend.submit(Some(&[damage])).unwrap();

		state.space.elements().for_each(|window| {
			window.send_frame(
				&self.output,
				state.start_time.elapsed(),
				Some(Duration::ZERO),
				|_, _| Some(self.output.clone()),
			);
		});

		state.space.refresh();
		state.popups.cleanup();
		let _ = state.display_handle.flush_clients();

		// Ask for redraw to schedule new frame.
		self.backend.window().request_redraw();
	}
}

impl MayState {
	fn handle_winit_event(&mut self, event: WinitEvent, wd: &mut WinitData) {
		match event {
			WinitEvent::Resized { size, .. } => {
				let mode = Mode {
					size,
					refresh: 60_000,
				};

				wd.output.change_current_state(Some(mode), None, None, None);
			}
			WinitEvent::Redraw => wd.render(self),
			WinitEvent::CloseRequested => self.loop_signal.stop(),
			WinitEvent::Input(input) => self.handle_input_event(input),
			event => println!("event {:?}", event),
		}
	}
}
