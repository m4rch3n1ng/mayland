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
	Workspaces,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "tag", content = "val")]
pub enum Response {
	Err(Error),
	Dispatch,
	Workspaces(Vec<Workspace>),
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
