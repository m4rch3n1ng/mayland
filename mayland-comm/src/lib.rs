pub use mayland_config::Action;
use serde::{Deserialize, Serialize};

pub const MAYLAND_SOCKET_VAR: &str = "MAYLAND_SOCKET";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Request {
	Dispatch(Action),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Response {
	Dispatch,
}
