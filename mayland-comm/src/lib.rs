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
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "tag", content = "val")]
pub enum Response {
	Dispatch,
	Info(Info),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
	pub workspaces: Vec<Workspace>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
	pub idx: usize,
	pub output: Option<String>,

	pub amt: usize,
	pub windows: Vec<Window>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Window {
	pub title: Option<String>,
	pub app_id: Option<String>,
}
