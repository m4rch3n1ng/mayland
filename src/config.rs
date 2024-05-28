use self::{bind::Binds, input::Input};
use crate::{error::MaylandError, state::State};
use annotate_snippets::{Level, Renderer, Snippet};
use serde::Deserialize;

mod bind;
mod input;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
	pub input: Input,
	pub bind: Binds,
}

const CONFIG_PATH: &str = "/home/may/.config/mayland.mf";

impl Config {
	pub fn read() -> Result<Self, MaylandError> {
		let file = std::fs::read_to_string(CONFIG_PATH)
			.map_err(|_| MaylandError::FileNotFound(CONFIG_PATH.to_owned()))?;

		// workaround for https://github.com/rust-lang/annotate-snippets-rs/issues/25
		let file = file.replace('\t', "    ");

		match mayfig::from_str(&file) {
			Ok(config) => Ok(config),
			Err(err) => {
				let code = err.code().to_string();
				let message = if let Some(span) = err.span() {
					Level::Error.title(&code).snippet(
						Snippet::source(&file)
							.origin(CONFIG_PATH)
							.fold(true)
							.annotation(Level::Error.span(span.range())),
					)
				} else {
					Level::Error.title(&code)
				};

				let renderer = Renderer::styled();
				anstream::println!("{}", renderer.render(message));

				Err(MaylandError::AlreadyPrinted)
			}
		}
	}
}

impl Config {
	pub fn init(&self, state: &mut State) {
		self.input.keyboard.load_file(state);
	}
}
