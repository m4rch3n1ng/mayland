use serde::{Deserialize, Serialize};

mod action;
mod error;

pub use self::action::Action;
pub use self::error::Error;

pub const MAYLAND_SOCKET_VAR: &str = "MAYLAND_SOCKET";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "tag", content = "val")]
pub enum Request {
	Dispatch(Action),
	Reload,
	Devices,
	Outputs,
	Workspaces,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "tag", content = "val")]
pub enum Response {
	Err(Error),
	Dispatch,
	Reload,
	Devices(Vec<Device>),
	Outputs(Vec<Output>),
	Workspaces(Vec<Workspace>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
	pub name: String,
	pub r#type: device::Type,
	pub vid: u32,
	pub pid: u32,
}

pub mod device {
	use super::Device;
	use serde::{Deserialize, Serialize};
	use std::fmt::Display;

	impl Display for Device {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			writeln!(f, "device {:?}", self.name)?;
			writeln!(f, "    type: {}", self.r#type)?;
			writeln!(f, "    vid: {:#06x}", self.vid)?;
			writeln!(f, "    pid: {:#06x}", self.pid)?;

			Ok(())
		}
	}

	#[derive(Debug, Serialize, Deserialize)]
	#[serde(rename_all = "kebab-case")]
	pub enum Type {
		Keyboard,
		Touchpad,
		Pointer,
		Touch,
		Tablet,
		TabletPad,
		Switch,
	}

	impl Display for Type {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			match self {
				Type::Keyboard => f.write_str("keyboard"),
				Type::Touchpad => f.write_str("touchpad"),
				Type::Pointer => f.write_str("pointer"),
				Type::Touch => f.write_str("touch"),
				Type::Tablet => f.write_str("tablet"),
				Type::TabletPad => f.write_str("tablet-pad"),
				Type::Switch => f.write_str("switch"),
			}
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
	pub name: String,
	pub mode: Option<output::Mode>,
	pub make: String,
	pub model: String,
	pub serial: Option<String>,
	pub size: Option<(u32, u32)>,
	pub logical: Option<output::Logical>,
	pub modes: Vec<output::Mode>,
}

pub mod output {
	use super::Output;
	use serde::{Deserialize, Serialize};
	use std::fmt::Display;

	impl Display for Output {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			writeln!(f, "output {:?}", self.name)?;
			if let Some(mode) = &self.mode {
				writeln!(f, "    mode: {}", mode)?;
			}

			if let Some(logical) = self.logical {
				writeln!(f, "    mapped at: {},{}", logical.x, logical.y)?;
				writeln!(f, "    mapped size: {}x{}", logical.w, logical.h)?;
				writeln!(f, "    mapped transform: {}", logical.transform)?;
			}

			writeln!(f, "    make: {}", self.make)?;
			writeln!(f, "    model: {}", self.model)?;
			if let Some(serial) = &self.serial {
				writeln!(f, "    serial: {}", serial)?;
			}

			if let Some((width, height)) = self.size {
				let inches = (width.pow(2) as f64 + height.pow(2) as f64).sqrt() / 25.4;
				writeln!(f, "    physical size: {}x{} mm ({:.3}\")", width, height, inches)?;
			}

			writeln!(f, "    available modes:")?;
			for mode in &self.modes {
				writeln!(f, "        {}", mode)?;
			}

			Ok(())
		}
	}

	#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
	pub struct Mode {
		pub w: u16,
		pub h: u16,
		pub refresh: u32,

		pub preferred: bool,
	}

	impl Display for Mode {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "{}x{}@{:.3}", self.w, self.h, self.refresh as f64 / 1000.)?;
			if self.preferred {
				write!(f, " (preferred)")?;
			}

			Ok(())
		}
	}

	#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
	pub struct Logical {
		pub x: i32,
		pub y: i32,
		pub w: i32,
		pub h: i32,
		pub transform: Transform,
		// scale
	}

	#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
	#[serde(rename_all = "snake_case")]
	pub enum Transform {
		Normal,
		#[serde(rename = "90")]
		_90,
		#[serde(rename = "180")]
		_180,
		#[serde(rename = "270")]
		_270,
		Flipped,
		#[serde(rename = "flipped_90")]
		Flipped90,
		#[serde(rename = "flipped_180")]
		Flipped180,
		#[serde(rename = "flipped_270")]
		Flipped270,
	}

	impl Display for Transform {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			match self {
				Transform::Normal => f.write_str("normal"),
				Transform::_90 => f.write_str("rotated 90° counter-clockwise"),
				Transform::_180 => f.write_str("rotated 180°"),
				Transform::_270 => f.write_str("rotated 270° counter-clockwise"),
				Transform::Flipped => f.write_str("flipped vertically"),
				Transform::Flipped90 => f.write_str("flipped vertically, rotated 90° counter-clockwise"),
				Transform::Flipped180 => f.write_str("flipped vertically, rotated 180°"),
				Transform::Flipped270 => f.write_str("flipped vertically, rotated 270° counter-clockwise"),
			}
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
	pub idx: usize,
	pub output: Option<String>,

	pub active: bool,
	pub windows: Vec<workspace::Window>,
}

pub mod workspace {
	use super::Workspace;
	use serde::{Deserialize, Serialize};
	use std::fmt::Display;

	impl Display for Workspace {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			if let Some(output) = &self.output {
				writeln!(f, "workspace {} @ {:?}", self.idx, output)?;
			} else {
				writeln!(f, "workspace {}", self.idx)?;
			}

			writeln!(f, "    active: {}", self.active)?;

			for window in &self.windows {
				match (&window.app_id, &window.title) {
					(Some(app_id), Some(title)) => writeln!(f, "    window {:?} @ {:?}", app_id, title)?,
					(Some(app_id), None) => writeln!(f, "    window {:?}", app_id)?,
					(None, Some(title)) => writeln!(f, "    window @ {:?}", title)?,
					(None, None) => writeln!(f, "    window")?,
				}
			}

			Ok(())
		}
	}

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Window {
		pub app_id: Option<String>,
		pub title: Option<String>,
	}
}
