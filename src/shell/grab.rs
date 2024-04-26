use super::element::WindowElement;
use crate::state::State;
use smithay::utils::Serial;

mod floating;

impl State {
	pub fn xdg_move(&mut self, window: WindowElement, serial: Serial) {
		self.xdg_floating_move(window, serial);
	}

	pub fn xdg_resize(&mut self, window: WindowElement, serial: Serial) {
		self.xdg_floating_resize(window, serial);
	}
}
