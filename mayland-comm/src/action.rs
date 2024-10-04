use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "tag", content = "val")]
pub enum Action {
	Quit,
	CloseWindow,

	Workspace(usize),

	Spawn(Vec<String>),
}
