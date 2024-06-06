use crate::State;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Input {
	pub xkb: Xkb,
}

#[derive(Debug, Default, Deserialize)]
pub struct Xkb {
	pub file: Option<String>,
}

impl Xkb {
	pub fn load_file(&self, state: &mut State) {
		if let Some(xkb_file) = self.file.as_deref() {
			let keymap = std::fs::read_to_string(xkb_file).unwrap();

			let xkb = state.mayland.seat.get_keyboard().unwrap();
			xkb.set_keymap_from_string(state, keymap).unwrap();
		}
	}
}
