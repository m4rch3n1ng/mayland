use mayland::{action::Action, comm::Event};
use postcard::to_allocvec;
use std::{io::Write, os::unix::net::UnixStream};

const SOCKET_PATH: &str = "/tmp/mayland.sock";

fn main() {
	let action = Action::Workspace(2);
	let event = Event::Dispatch(action);

	let v = to_allocvec(&event).unwrap();

	let mut unix_stream = UnixStream::connect(SOCKET_PATH).unwrap();
	unix_stream.write_all(&v).unwrap();
}
