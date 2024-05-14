use self::{bind::Binds, input::Input};
use crate::state::State;
use serde::Deserialize;

mod bind;
mod input;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
	pub input: Input,
	pub bind: Binds,
}

const CONFIG_PATH: &str = "/home/may/.config/mayland.mf";

impl Config {
	pub fn read() -> Self {
		let file = std::fs::read_to_string(CONFIG_PATH).unwrap();
		mayfig::from_str(&file).unwrap()
	}
}

impl State {
	pub fn init_conf(&mut self) {
		if let Some(xkb_file) = self.mayland.config.input.xkb.file.as_deref() {
			let xkb = self.mayland.seat.get_keyboard().unwrap();
			let keymap = std::fs::read_to_string(xkb_file).unwrap();
			xkb.set_keymap_from_string(self, keymap).unwrap();
		}
	}
}