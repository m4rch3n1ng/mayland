use annotate_snippets::{Level, Renderer, Snippet};
use bind::CompMod;
use serde::Deserialize;
use std::{path::PathBuf, sync::LazyLock};

pub mod bind;
pub mod decoration;
mod error;
pub mod input;
pub mod layout;
pub mod outputs;

pub use self::{
	bind::Binds, decoration::Decoration, error::Error, input::Input, layout::Layout, outputs::Outputs,
};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
	pub input: Input,
	pub output: Outputs,
	pub cursor: Cursor,
	pub decoration: Decoration,
	pub layout: Layout,
	pub bind: Binds,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Cursor {
	pub xcursor_theme: Option<String>,
	pub xcursor_size: Option<u32>,
}

static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
	let mut config = dirs::config_dir().unwrap();
	config.push("mayland.mf");

	config
});

impl Config {
	pub fn read(comp: CompMod) -> Result<Self, Error> {
		let file = match std::fs::read_to_string(&*CONFIG_PATH) {
			Ok(file) => file,
			Err(err) if matches!(err.kind(), std::io::ErrorKind::NotFound) => {
				let mut config = Config::default();
				config.bind = config.bind.flatten_mod(comp);

				return Ok(config);
			}
			Err(err) => return Err(From::from(err)),
		};

		// workaround for https://github.com/rust-lang/annotate-snippets-rs/issues/25
		let file = file.replace('\t', "    ");

		match mayfig::from_str::<Config>(&file) {
			Ok(mut config) => {
				config.bind = config.bind.flatten_mod(comp);
				Ok(config)
			}
			Err(err) => {
				let code = err.code().to_string();
				let path = &*CONFIG_PATH.to_string_lossy();

				let message = if let Some(span) = err.span() {
					Level::Error.title(&code).snippet(
						Snippet::source(&file)
							.origin(path)
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
