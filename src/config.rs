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

impl Config {
	pub fn init(&self, state: &mut State) {
		self.input.xkb.load_file(state);
	}
}
