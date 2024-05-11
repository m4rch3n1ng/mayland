use crate::{action::Action, state::Mayland, State};
use serde::{Deserialize, Serialize};
pub use socket::MaySocket;
use std::{io::Write, os::unix::net::UnixStream};

pub mod socket;

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

pub fn init(mayland: &mut Mayland) {
	let socket = MaySocket::init();
	mayland
		.loop_handle
		.insert_source(socket, |event, stream, state| {
			state.handle_socket_event(event, stream);
		})
		.unwrap();
}

impl State {
	fn handle_socket_event(&mut self, event: Event, stream: &mut UnixStream) {
		match event {
			Event::Dispatch(action) => self.handle_action(action),
			Event::Info => {
				let info = Info::new(self);
				let wire = postcard::to_stdvec(&info).unwrap();
				stream.write_all(&wire).unwrap();
			}
		}
	}
}
