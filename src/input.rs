use crate::{shell::focus::PointerFocusTarget, state::State};
use smithay::{
	backend::{
		input::{Event, InputEvent, KeyboardKeyEvent},
		winit::WinitInput,
	},
	desktop::layer_map_for_output,
	input::keyboard::FilterResult,
	utils::{Logical, Point, SERIAL_COUNTER},
};

impl State {
	pub fn handle_input_event(&mut self, event: InputEvent<WinitInput>) {
		match event {
			InputEvent::Keyboard { event, .. } => {
				let keyboard = self.seat.get_keyboard().unwrap();

				let code = event.key_code();
				let state = event.state();
				let serial = SERIAL_COUNTER.next_serial();
				let time = event.time_msec();

				let _ = keyboard.input(self, code, state, serial, time, |_state, _mods, keysym| {
					let raw_sym = keysym.raw_syms()[0];
					println!("key {:?}", raw_sym);

					FilterResult::Forward::<()>
				});
			}
			InputEvent::PointerMotion { .. } => {}
			InputEvent::PointerMotionAbsolute { .. } => {}
			_ => println!("input {:?}", event),
		}
	}

	pub fn surface_under(
		&self,
		pos: Point<f64, Logical>,
	) -> Option<(PointerFocusTarget, Point<i32, Logical>)> {
		let output = self.space.outputs().find(|output| {
			let geometry = self.space.output_geometry(output).unwrap();
			geometry.contains(pos.to_i32_round())
		})?;
		let output_geo = self.space.output_geometry(output).unwrap();
		let layers = layer_map_for_output(output);

		todo!()
	}
}
