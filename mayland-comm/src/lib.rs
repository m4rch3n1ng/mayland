//! shared types for socket communication with mayland
//!
//! you can communicate with mayland over a unix domain socket. if you are running in a mayland
//! instance, mayland exposes the path to the mayland socket in the environment variable
//! [`MAYLAND_SOCKET_VAR`].
//!
//! to communicate with mayland, you have to send a [`Request`] and then mayland will send
//! a [`Response`] back.
//!
//! all requests are sent as json, where the request is all on a single line to allow for future
//! request batching.
//! this is easily done with the default [`serde_json`](https://crates.io/crates/serde_json)
//! Serializer, which already serializes the json content into a single line.
//!
//! all enums that have values are in the serde
//! [adjacently tagged](https://serde.rs/enum-representations.html#adjacently-tagged)
//! enum representation, with the tag being `"tag"` and the content being `"val"`, and
//! everything being in `snake_case`.

#![warn(missing_docs)]

use serde::{Deserialize, Serialize};

mod action;
mod error;

pub use self::action::Action;
pub use self::error::Error;

/// the environment variable, where the path to the mayland socket is found.
///
/// this environment variable is available when inside of mayland.
pub const MAYLAND_SOCKET_VAR: &str = "MAYLAND_SOCKET";

/// send a request to mayland
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "tag", content = "val")]
pub enum Request {
	/// dispatch an action for mayland to handle
	///
	/// ```json
	/// { "tag": "dispatch", "val": { "tag": "quit" }}
	/// ```
	Dispatch(Action),
	/// request mayland to reload the config
	///
	/// ```json
	/// { "tag": "reload" }
	/// ```
	Reload,
	/// requset device info from mayland
	///
	/// ```json
	/// { "tag": "reload" }
	/// ```
	Devices,
	/// request output info from mayland
	///
	/// ```json
	/// { "tag": "outputs" }
	/// ```
	Outputs,
	/// request window info from mayland
	///
	/// ```json
	/// { "tag": "windows" }
	/// ```
	Windows,
	/// request workspace info from mayland
	///
	/// ```json
	/// { "tag": "workspaces" }
	/// ```
	Workspaces,
}

/// the response that mayland sends back
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "tag", content = "val")]
pub enum Response {
	/// mayland returned an error
	Err(Error),
	/// mayland successfully handled the dispatch request
	Dispatch,
	/// mayland successfully handled the reload request
	Reload,
	/// mayland device info
	Devices(Vec<Device>),
	/// mayland output info
	Outputs(Vec<Output>),
	/// mayland window info
	Windows(Vec<Window>),
	/// mayland workspace info
	Workspaces(Vec<Workspace>),
}

/// an input device registered in mayland
#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
	/// device name
	pub name: String,
	/// device type
	pub r#type: device::Type,
	/// device vendor id
	pub vid: u32,
	/// device product id
	pub pid: u32,
}

pub mod device {
	//! mayland device extra info

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

	/// device type
	#[derive(Debug, Serialize, Deserialize)]
	#[serde(rename_all = "kebab-case")]
	pub enum Type {
		/// a keyboard device
		Keyboard,
		/// a touchpad device
		///
		/// touchpad is, according to libinput, not a seperate device from a
		/// regular pointer type that has a finger tab counter bigger than 0
		Touchpad,
		/// a pointer device
		Pointer,
		/// a touch device
		Touch,
		/// a tablet device
		Tablet,
		/// a tablet-pad
		TabletPad,
		/// a switch device
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

/// a mayland logical output
#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
	/// output connector
	pub name: String,
	/// output display mode
	pub mode: Option<output::Mode>,
	/// output make
	pub make: String,
	/// output model
	pub model: String,
	/// ouput serial
	pub serial: Option<String>,
	/// physical output size in mm
	pub size: Option<(u32, u32)>,
	/// logical information of the output
	pub logical: Option<output::Logical>,
	/// available display modes
	pub modes: Vec<output::Mode>,
}

pub mod output {
	//! mayland output extra info

	use super::Output;
	use serde::{Deserialize, Serialize};
	use std::fmt::Display;

	impl Display for Output {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			writeln!(f, "output {:?}", self.name)?;
			if let Some(mode) = &self.mode {
				writeln!(f, "    mode: {mode}")?;
			}

			if let Some(logical) = self.logical {
				writeln!(f, "    mapped at: {},{}", logical.x, logical.y)?;
				writeln!(f, "    mapped size: {}x{}", logical.w, logical.h)?;
				writeln!(f, "    mapped transform: {}", logical.transform)?;
			}

			writeln!(f, "    make: {}", self.make)?;
			writeln!(f, "    model: {}", self.model)?;
			if let Some(serial) = &self.serial {
				writeln!(f, "    serial: {serial}")?;
			}

			if let Some((width, height)) = self.size {
				let inches = (width.pow(2) as f64 + height.pow(2) as f64).sqrt() / 25.4;
				writeln!(f, "    physical size: {width}x{height} mm ({inches:.3}\")")?;
			}

			writeln!(f, "    available modes:")?;
			for mode in &self.modes {
				writeln!(f, "        {mode}")?;
			}

			Ok(())
		}
	}

	/// display mode
	#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
	pub struct Mode {
		/// udev mode height
		pub w: u16,
		/// udev mode width
		pub h: u16,
		/// refresh rate in ms
		pub refresh: u32,

		/// is the mode preferred?
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

	/// logical output information
	#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
	pub struct Logical {
		/// logical x position
		pub x: i32,
		/// logical y position
		pub y: i32,
		/// width in logical px
		pub w: i32,
		/// height in logical px
		pub h: i32,
		/// output transform
		pub transform: Transform,
		// scale
	}

	/// output transform
	#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
	#[serde(rename_all = "snake_case")]
	pub enum Transform {
		/// normal
		Normal,
		/// rotated 90° counter-clockwise
		#[serde(rename = "90")]
		_90,
		/// rotated 180°
		#[serde(rename = "180")]
		_180,
		/// rotated 270° counter-clockwise
		#[serde(rename = "270")]
		_270,
		/// flipped vertically
		Flipped,
		/// flipped vertically, rotated 90° counter-clockwise
		#[serde(rename = "flipped_90")]
		Flipped90,
		/// flipped vertically, rotated 180°
		#[serde(rename = "flipped_180")]
		Flipped180,
		/// flipped vertically, rotated 270° counter-clockwise
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

/// a mayland window
#[derive(Debug, Serialize, Deserialize)]
pub struct Window {
	/// relative window geometry
	///
	/// relative, as in relative to the workspace
	pub relative: window::Geometry,
	/// absolute window geometry
	///
	/// may be `None` if the window is on an orphaned
	/// workspace or no output is connected.
	pub absolute: Option<window::Geometry>,

	/// the window app id
	///
	/// x11 calls this "class"
	pub app_id: Option<String>,
	/// the window title
	pub title: Option<String>,
	/// the window process id
	pub pid: Option<i32>,

	/// the workspace this window is mapped on
	pub workspace: usize,
	/// if the window is currently focussed
	pub active: bool,

	/// if the window is running under xwayland
	pub xwayland: bool,
}

pub mod window {
	//! mayland window extra info

	use super::Window;
	use serde::{Deserialize, Serialize};
	use std::fmt::Display;

	impl Display for Window {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			if let Some(app_id) = &self.app_id {
				writeln!(f, "window {app_id:?}")?;
			} else {
				writeln!(f, "window")?;
			}

			let geometry = self.absolute.as_ref().unwrap_or(&self.relative);
			writeln!(f, "    at: {},{}", geometry.x, geometry.y)?;
			writeln!(f, "    size: {}x{}", geometry.w, geometry.h)?;

			if let Some(app_id) = &self.app_id {
				writeln!(f, "    app_id: {app_id:?}")?;
			}
			if let Some(title) = &self.title {
				writeln!(f, "    title: {title:?}")?;
			}

			writeln!(f, "    workspace: {}", self.workspace)?;
			writeln!(f, "    active: {}", self.active)?;
			writeln!(f, "    xwayland: {}", self.xwayland)?;

			Ok(())
		}
	}

	/// the window geometry
	#[derive(Debug, Deserialize, Serialize)]
	pub struct Geometry {
		/// x
		pub x: i32,
		/// y
		pub y: i32,
		/// width
		pub w: i32,
		/// height
		pub h: i32,
	}
}

/// a mayland workspace
#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
	/// the index of the workspace
	pub idx: usize,
	/// the output the workspace is mapped on
	///
	/// this is None, when no outputs exist,
	/// or if the workspace is orphaned after
	/// its output was removed.
	pub output: Option<String>,

	/// is the workspace currently focussed
	pub active: bool,
	/// the windows that are mapped on the workspace
	pub windows: Vec<workspace::Window>,
}

pub mod workspace {
	//! mayland workspace extra info

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
					(Some(app_id), Some(title)) => writeln!(f, "    window {app_id:?} @ {title:?}")?,
					(Some(app_id), None) => writeln!(f, "    window {app_id:?}")?,
					(None, Some(title)) => writeln!(f, "    window @ {title:?}")?,
					(None, None) => writeln!(f, "    window")?,
				}
			}

			Ok(())
		}
	}

	/// a workspace window
	#[derive(Debug, Serialize, Deserialize)]
	pub struct Window {
		/// the `app_id` of the window
		pub app_id: Option<String>,
		/// the `title` of the window
		pub title: Option<String>,
	}
}
