use serde::{Deserialize, Serialize};

mod action;

pub use self::action::Action;

pub const MAYLAND_SOCKET_VAR: &str = "MAYLAND_SOCKET";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "tag", content = "val")]
pub enum Request {
	Dispatch(Action),
	Info,
	Workspaces,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "tag", content = "val")]
pub enum Response {
	Dispatch,
	Info(Info),
	Workspaces(Vec<Workspace>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
	pub workspaces: Vec<Workspace>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
	pub idx: usize,
	pub output: Option<String>,
	pub windows: Vec<workspace::Window>,
}

pub mod workspace {
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Window {
		pub title: Option<String>,
		pub app_id: Option<String>,
	}
}
