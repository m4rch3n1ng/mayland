use serde::{Deserialize, Serialize};

/// an action to send to the compositor
///
/// the [`Action`] is placed into the `"val"` field of [`Request::Dispatch`](super::Request::Dispatch).
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "tag", content = "val")]
pub enum Action {
	/// quit the compositor
	///
	/// ```json
	/// { "tag": "quit" }
	/// ```
	Quit,

	/// close the currently focussed window
	///
	/// ```json
	/// { "tag": "close_window" }
	/// ```
	CloseWindow,
	/// toggle the floating state of the currently focussed window
	///
	/// ```json
	/// { "tag": "toggle_floating" }
	/// ```
	ToggleFloating,

	/// switch to a workspace
	///
	/// ```json
	/// { "tag": "workspace", "val": 2 }
	/// ```
	Workspace(usize),

	/// spawn a command
	///
	/// ```json
	/// { "tag": "spawn", "val": [ "kitty" ]}
	/// ```
	Spawn(Vec<String>),
}
