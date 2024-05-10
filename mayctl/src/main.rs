use mayland::{action::Action, comm::Event};
use std::{io::Write, os::unix::net::UnixStream};

const SOCKET_PATH: &str = "/tmp/mayland.sock";

fn main() {
	let action = Action::Workspace(2);
	let event = Event::Dispatch(action);

	let v = postcard::to_stdvec(&event).unwrap();

	let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
	stream.write_all(&v).unwrap();
}
