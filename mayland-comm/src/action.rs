use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
	Quit,
	#[serde(alias = "close")]
	CloseWindow,

	Workspace(usize),

	Spawn(Vec<String>),
}
