use bind::CompMod;
use error::MayfigError;
use serde::Deserialize;
use std::{path::PathBuf, sync::LazyLock};

pub mod bind;
pub mod decoration;
pub mod error;
pub mod input;
pub mod layout;
pub mod outputs;
pub mod windowrules;

pub use self::{
	bind::Binds, decoration::Decoration, error::Error, input::Input, layout::Layout, outputs::Outputs,
	windowrules::WindowRules,
};

#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Config {
	pub input: Input,
	pub output: Outputs,
	pub cursor: Cursor,
	pub decoration: Decoration,
	pub layout: Layout,
	pub bind: Binds,
	pub windowrules: WindowRules,
}

#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
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
	pub fn init(comp: CompMod) -> Result<Self, Error> {
		match Config::read(comp) {
			Ok(config) => Ok(config),
			Err(Error::NotFound) => {
				let mut config = Config::default();
				config.bind = config.bind.flatten_mod(comp);

				Ok(config)
			}
			Err(e) => Err(e),
		}
	}

	pub fn read(comp: CompMod) -> Result<Self, Error> {
		let file = match std::fs::read_to_string(&*CONFIG_PATH) {
			Ok(file) => file,
			Err(err) if matches!(err.kind(), std::io::ErrorKind::NotFound) => return Err(Error::NotFound),
			Err(err) => return Err(Error::IoError(err)),
		};

		// workaround for https://github.com/rust-lang/annotate-snippets-rs/issues/25
		let file = file.replace('\t', "    ");

		let mut config = mayfig::from_str::<Config>(&file).map_err(|error| MayfigError { error, file })?;
		config.bind = config.bind.flatten_mod(comp);
		Ok(config)
	}
}
