use crate::{action::Action, State};
use serde::{Deserialize, Serialize};

pub mod socket;

pub use socket::MaySocket;

#[derive(Debug, Deserialize, Serialize)]
pub enum Event {
	Dispatch(Action),
	Info,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Info {
	workspaces: Vec<usize>,
}

impl Info {
	fn new(state: &State) -> Self {
		let workspaces = state.mayland.workspaces.workspace_indices().collect();

		Info { workspaces }
	}
}
