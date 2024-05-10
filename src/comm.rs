use crate::{action::Action, state::Mayland, State};
use serde::{Deserialize, Serialize};
pub use socket::MaySocket;

pub mod socket;

#[derive(Debug, Deserialize, Serialize)]
pub enum Event {
	Dispatch(Action),
}

pub fn init(mayland: &mut Mayland) {
	let socket = MaySocket::init();
	mayland
		.loop_handle
		.insert_source(socket, |event, (), state| {
			state.handle_socket_event(event);
		})
		.unwrap();
}

impl State {
	fn handle_socket_event(&mut self, event: Event) {
		match event {
			Event::Dispatch(action) => self.handle_action(action),
		}
	}
}
