use bind::CompMod;
use error::MayfigError;
use serde::{Deserialize, de::Visitor};
use smithay::reexports::calloop;
use std::{
	collections::{HashMap, HashSet},
	ops::Deref,
	path::{Path, PathBuf},
	sync::OnceLock,
	time::Duration,
};

pub mod bind;
pub mod decoration;
pub mod error;
pub mod input;
pub mod layout;
pub mod outputs;
pub mod windowrules;

pub use self::{
	bind::{Action, Binds},
	decoration::Decoration,
	error::Error,
	input::Input,
	layout::Layout,
	outputs::Outputs,
	windowrules::WindowRules,
};

#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
	pub input: Input,
	pub output: Outputs,
	pub cursor: Cursor,
	pub decoration: Decoration,
	pub layout: Layout,
	#[serde(rename = "env")]
	pub environment: Environment,
	pub bind: Binds,
	pub windowrules: WindowRules,
}

#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Cursor {
	pub xcursor_theme: Option<String>,
	pub xcursor_size: Option<u32>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Environment {
	pub envs: HashMap<String, String>,
	pub remove: HashSet<String>,
}

impl<'de> Deserialize<'de> for Environment {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		deserializer.deserialize_map(EnvVis)
	}
}

struct EnvVis;

impl<'a> Visitor<'a> for EnvVis {
	type Value = Environment;

	fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("environment variables")
	}

	fn visit_map<A: serde::de::MapAccess<'a>>(self, mut map: A) -> Result<Self::Value, A::Error> {
		let mut environment = Environment::default();

		while let Some((key, val)) = map.next_entry::<String, String>()? {
			if val.is_empty() {
				environment.envs.remove(&key);
				environment.remove.insert(key);
			} else {
				environment.remove.remove(&key);
				environment.envs.insert(key, val);
			}
		}

		Ok(environment)
	}
}

pub struct LazyOnceLock<T, F = fn() -> T> {
	once: OnceLock<T>,
	init: F,
}

impl<T, F: Fn() -> T> LazyOnceLock<T, F> {
	const fn new(init: F) -> Self {
		LazyOnceLock {
			once: OnceLock::new(),
			init,
		}
	}

	fn set(&self, v: T) -> Result<(), T> {
		self.once.set(v)
	}
}

impl<T, F: Fn() -> T> Deref for LazyOnceLock<T, F> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		self.once.get_or_init(|| (self.init)())
	}
}

pub static CONFIG_PATH: LazyOnceLock<PathBuf> = LazyOnceLock::new(|| {
	let mut config = dirs::config_dir().unwrap();
	config.push("mayland.mf");

	config
});

const DEFAULT_CONFIG: &str = include_str!("../../resources/mayland.mf");

impl Config {
	pub fn init(
		comp: CompMod,
		config: Option<PathBuf>,
	) -> Result<(Self, calloop::channel::Channel<Self>), Error> {
		if let Some(config) = config {
			if !config.exists() {
				return Err(Error::NotFound(config));
			}

			let _ = CONFIG_PATH.set(config);
		}

		let config = match Config::read(&CONFIG_PATH, comp) {
			Ok(config) => Ok(config),
			Err(Error::NotFound(_)) => {
				let mut config = Config::default();
				config.bind = config.bind.flatten_mod(comp);

				match std::fs::write(&*CONFIG_PATH, DEFAULT_CONFIG) {
					Ok(()) => tracing::info!("created default config at {:?}", &*CONFIG_PATH),
					Err(err) => tracing::warn!("failed to create config file at {:?} ({err})", &*CONFIG_PATH),
				}

				Ok(config)
			}
			Err(e) => Err(e),
		}?;

		let rx = watcher(comp);
		Ok((config, rx))
	}

	pub fn read(path: &Path, comp: CompMod) -> Result<Self, Error> {
		let file = match std::fs::read_to_string(path) {
			Ok(file) => file,
			Err(err) if matches!(err.kind(), std::io::ErrorKind::NotFound) => {
				return Err(Error::NotFound(path.to_owned()));
			}
			Err(err) => return Err(Error::IoError(path.to_owned(), err)),
		};

		// workaround for https://github.com/rust-lang/annotate-snippets-rs/issues/25
		let file = file.replace('\t', "    ");
		let mut config = mayfig::from_str::<Config>(&file).map_err(|error| MayfigError {
			error: Box::new(error),
			path: path.to_owned(),
			file,
		})?;

		config.bind = config.bind.flatten_mod(comp);
		Ok(config)
	}
}

/// initialize config watcher
fn watcher(comp: CompMod) -> calloop::channel::Channel<Config> {
	let (tx, rx) = calloop::channel::sync_channel(1);
	std::thread::spawn(move || {
		let mut mtime = CONFIG_PATH.metadata().and_then(|stat| stat.modified()).ok();

		loop {
			let second = Duration::from_secs(1);
			std::thread::sleep(second);

			let Ok(stat) = CONFIG_PATH.metadata().and_then(|stat| stat.modified()) else {
				mtime = None;
				continue;
			};

			if mtime.is_some_and(|mtime| mtime == stat) {
				continue;
			}

			mtime = Some(stat);

			if let Ok(config) = Config::read(&CONFIG_PATH, comp)
				&& tx.send(config).is_err()
			{
				tracing::warn!("file watch channel closed unexpectedly?");
				break;
			}
		}
	});

	rx
}

#[cfg(test)]
mod test {
	use super::{Config, DEFAULT_CONFIG};

	#[test]
	fn default_config() {
		let mayland_mf = mayfig::from_str::<Config>(DEFAULT_CONFIG).unwrap();
		let default = Config::default();
		pretty_assertions::assert_eq!(mayland_mf, default);
	}
}
