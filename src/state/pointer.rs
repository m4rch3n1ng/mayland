use super::State;
use smithay::{
	delegate_pointer_gestures, delegate_relative_pointer,
	input::pointer::MotionEvent,
	utils::{Logical, Point, SERIAL_COUNTER},
};

impl State {
	pub fn move_cursor(&mut self, location: Point<f64, Logical>) {
		let pointer = self.mayland.pointer.clone();
		let under = self.surface_under(location);

		let serial = SERIAL_COUNTER.next_serial();
		let time = self.mayland.clock.now().as_millis();

		pointer.motion(
			self,
			under.clone(),
			&MotionEvent {
				location,
				serial,
				time,
			},
		);
	}

	pub fn reset_keyboard_focus(&mut self) {
		let serial = SERIAL_COUNTER.next_serial();
		let workspace = self.mayland.workspaces.workspace();
		if workspace.is_empty() {
			let keyboard = self.mayland.keyboard.clone();
			keyboard.set_focus(self, None, serial);
		} else {
			let location = self.mayland.pointer.current_location();
			self.update_keyboard_focus(location, serial);
		}
	}
}

delegate_relative_pointer!(State);
delegate_pointer_gestures!(State);
