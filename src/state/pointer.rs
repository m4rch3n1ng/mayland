use super::State;
use crate::{layout::Relocate, shell::focus::KeyboardFocusTarget};
use smithay::{
	delegate_pointer_gestures, delegate_relative_pointer,
	desktop::LayerSurface,
	input::pointer::MotionEvent,
	utils::{Logical, Point, SERIAL_COUNTER},
};

impl State {
	pub fn relocate(&mut self, relocate: Relocate) {
		match relocate {
			Relocate::Absolute(location) => {
				self.move_cursor(location.to_f64());
			}
			Relocate::Relative(relative) => {
				let current = self.mayland.pointer.current_location();
				let location = current + relative.to_f64();
				self.move_cursor(location);
			}
		}

		self.mayland.queue_redraw_all();
	}

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
		pointer.frame(self);
	}

	pub fn focus_layer_surface(&mut self, surface: LayerSurface) {
		let serial = SERIAL_COUNTER.next_serial();
		let keyboard = self.mayland.keyboard.clone();

		keyboard.set_focus(self, Some(KeyboardFocusTarget::LayerSurface(surface)), serial);
		self.refresh_pointer_focus();
	}

	/// resets the keyboard and pointer focus
	pub fn reset_focus(&mut self) {
		let serial = SERIAL_COUNTER.next_serial();

		let workspace = self.mayland.workspaces.workspace();
		if workspace.is_none_or(|ws| ws.is_empty()) {
			let keyboard = self.mayland.keyboard.clone();
			keyboard.set_focus(self, None, serial);
		} else {
			let location = self.mayland.pointer.current_location();
			self.update_keyboard_focus(location, serial);
		}

		self.refresh_pointer_focus();
	}

	/// refresh pointer focus if needed
	///
	/// checks if the `pointer::current_focus` is equal to
	/// `surface_under(pointer::location)`, and if not issues an empty
	/// `pointer::motion` event to refresh the focus
	pub fn refresh_pointer_focus(&mut self) {
		let location = self.mayland.pointer.current_location();

		let curr = self.surface_under(location);
		let prev = self.mayland.pointer.current_focus();

		if curr.as_ref().map(|a| &a.0) == prev.as_ref() {
			return;
		}

		let pointer = self.mayland.pointer.clone();
		pointer.motion(
			self,
			curr,
			&MotionEvent {
				location,
				serial: SERIAL_COUNTER.next_serial(),
				time: self.mayland.clock.now().as_millis(),
			},
		);
		pointer.frame(self);
	}
}

delegate_relative_pointer!(State);
delegate_pointer_gestures!(State);
