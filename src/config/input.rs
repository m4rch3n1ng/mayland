use crate::State;
use serde::Deserialize;
use smithay::input::keyboard::XkbConfig;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Input {
	pub keyboard: Keyboard,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Keyboard {
	pub xkb_file: Option<String>,

	pub xkb_rules: String,
	pub xkb_model: String,
	pub xkb_layout: String,
	pub xkb_variant: String,
	pub xkb_options: Option<String>,

	pub repeat_delay: i32,
	pub repeat_rate: i32,
}

impl Keyboard {
	pub fn load_file(&self, state: &mut State) {
		if let Some(xkb_file) = self.xkb_file.as_deref() {
			let keymap = std::fs::read_to_string(xkb_file).unwrap();

			let xkb = state.mayland.seat.get_keyboard().unwrap();
			xkb.set_keymap_from_string(state, keymap).unwrap();
		}
	}

	pub fn xkb_config(&self) -> XkbConfig {
		XkbConfig {
			rules: &self.xkb_rules,
			model: &self.xkb_model,
			layout: &self.xkb_layout,
			variant: &self.xkb_variant,
			options: self.xkb_options.clone(),
		}
	}
}

impl Default for Keyboard {
	fn default() -> Self {
		Keyboard {
			xkb_file: None,

			xkb_rules: String::new(),
			xkb_model: String::new(),
			xkb_layout: String::new(),
			xkb_variant: String::new(),
			xkb_options: None,

			repeat_delay: 600,
			repeat_rate: 25,
		}
	}
}
