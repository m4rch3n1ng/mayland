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
	Outputs(Vec<Output>),
	Workspaces(Vec<Workspace>),
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
		// transform
		// scale
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
