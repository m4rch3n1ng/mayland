use annotate_snippets::{Level, Renderer, Snippet};
use bind::CompMod;
use serde::Deserialize;

mod action;
pub mod bind;
mod error;
pub mod input;

pub use self::{action::Action, bind::Binds, error::Error, input::Input};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
	pub input: Input,
	pub bind: Binds,
}

const CONFIG_PATH: &str = "/home/may/.config/mayland.mf";

impl Config {
	pub fn read(comp: CompMod) -> Result<Self, Error> {
		let file =
			std::fs::read_to_string(CONFIG_PATH).map_err(|_| Error::FileNotFound(CONFIG_PATH.to_owned()))?;

		// workaround for https://github.com/rust-lang/annotate-snippets-rs/issues/25
		let file = file.replace('\t', "    ");

		match mayfig::from_str::<Config>(&file) {
			Ok(mut config) => {
				config.bind = config.bind.flatten_mod(comp);
				Ok(config)
			}
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

				Err(Error::AlreadyPrinted)
			}
		}
	}
}
