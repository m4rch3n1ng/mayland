use crate::state::Mayland;
use smithay::reexports::calloop::{generic::Generic, Interest, Mode, PostAction};
use std::{
	io::{BufRead, BufReader},
	os::unix::net::{UnixListener, UnixStream},
};

static SOCKET_PATH: &str = "/tmp/mayland.sock";

pub struct MaySocket {
	pub path: String,
}

impl MaySocket {
	pub fn init(mayland: &Mayland) -> Self {
		let socket_path = SOCKET_PATH.to_owned();

		let listener = UnixListener::bind(SOCKET_PATH).unwrap();
		listener.set_nonblocking(true).unwrap();

		let source = Generic::new(listener, Interest::BOTH, Mode::Level);
		mayland
			.loop_handle
			.insert_source(source, |_, socket, _state| {
				match socket.accept() {
					Ok((stream, _addr)) => do_thing(stream),
					Err(io_err) if io_err.kind() == std::io::ErrorKind::WouldBlock => (),
					Err(e) => return Err(e),
				}

				Ok(PostAction::Continue)
			})
			.unwrap();

		MaySocket { path: socket_path }
	}
}

fn do_thing(stream: UnixStream) {
	let mut stream = BufReader::new(stream);

	let mut buf = String::new();
	stream.read_line(&mut buf).unwrap();

	dbg!(buf);
}

impl Drop for MaySocket {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(SOCKET_PATH);
	}
}
