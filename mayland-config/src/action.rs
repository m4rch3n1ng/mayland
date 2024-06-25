use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
	Quit,
	#[serde(alias = "close")]
	CloseWindow,

	Workspace(usize),

	Spawn(String),
}
