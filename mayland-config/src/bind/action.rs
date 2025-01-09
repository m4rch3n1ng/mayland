use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
	Quit,

	#[serde(alias = "close")]
	CloseWindow,
	ToggleFloating,

	Workspace(usize),

	Spawn(Vec<String>),
}

impl From<Action> for mayland_comm::Action {
	fn from(action: Action) -> Self {
		match action {
			Action::Quit => mayland_comm::Action::Quit,

			Action::CloseWindow => mayland_comm::Action::CloseWindow,
			Action::ToggleFloating => mayland_comm::Action::ToggleFloating,

			Action::Workspace(workspace) => mayland_comm::Action::Workspace(workspace),

			Action::Spawn(spawn) => mayland_comm::Action::Spawn(spawn),
		}
	}
}

impl From<mayland_comm::Action> for Action {
	/// this implementation is not strictly necessary and should
	/// probably not be used, but it exists so that the compiler warns
	/// if the two enums are out of sync
	fn from(action: mayland_comm::Action) -> Self {
		match action {
			mayland_comm::Action::Quit => Action::Quit,

			mayland_comm::Action::CloseWindow => Action::CloseWindow,
			mayland_comm::Action::ToggleFloating => Action::ToggleFloating,

			mayland_comm::Action::Workspace(workspace) => Action::Workspace(workspace),

			mayland_comm::Action::Spawn(spawn) => Action::Spawn(spawn),
		}
	}
}
