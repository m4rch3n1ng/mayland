use crate::state::MayState;
use smithay::{
	backend::{
		input::{Event, InputEvent, KeyboardKeyEvent},
		winit::WinitInput,
	},
	input::keyboard::FilterResult,
	utils::SERIAL_COUNTER,
};

impl MayState {
	pub fn handle_input_event(&mut self, event: InputEvent<WinitInput>) {
		match event {
			InputEvent::Keyboard { event, .. } => {
				let keyboard = self.seat.get_keyboard().unwrap();

				let code = event.key_code();
				let state = event.state();
				let serial = SERIAL_COUNTER.next_serial();
				let time = event.time_msec();

				let _ = keyboard.input(self, code, state, serial, time, |_, _, _| {
					FilterResult::Forward::<()>
				});
			}
			InputEvent::PointerMotion { .. } => {}
			InputEvent::PointerMotionAbsolute { .. } => {}
			_ => println!("input {:?}", event),
		}
	}
}
