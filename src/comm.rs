use crate::{state::Mayland, State};
use mayland_comm::{Request, Response};
use smithay::reexports::calloop::{generic::Generic, Interest, Mode, PostAction};
use std::{
	io::{BufRead, BufReader, Write},
	os::unix::net::{UnixListener, UnixStream},
};

static SOCKET_PATH: &str = "/tmp/mayland.sock";

pub struct MaySocket {
	pub path: String,
}

impl MaySocket {
	pub fn init(mayland: &Mayland) -> Self {
		let socket_path = SOCKET_PATH.to_owned();
		if std::fs::exists(&socket_path).unwrap() {
			std::fs::remove_file(&socket_path).unwrap();
		}

		let listener = UnixListener::bind(SOCKET_PATH).unwrap();
		listener.set_nonblocking(true).unwrap();

		let source = Generic::new(listener, Interest::READ, Mode::Level);
		mayland
			.loop_handle
			.insert_source(source, |_, socket, state| {
				match socket.accept() {
					Ok((stream, _addr)) => handle_stream(state, stream),
					Err(io_err) if io_err.kind() == std::io::ErrorKind::WouldBlock => (),
					Err(e) => return Err(e),
				}

				Ok(PostAction::Continue)
			})
			.unwrap();

		MaySocket { path: socket_path }
	}
}

fn handle_stream(state: &mut State, mut stream: UnixStream) {
	let mut read = BufReader::new(&mut stream);
	let mut buf = String::new();
	read.read_line(&mut buf).unwrap();

	let request = serde_json::from_str::<Request>(&buf).unwrap();
	let reply = match request {
		Request::Dispatch(action) => {
			state.handle_action(action);
			Response::Dispatch
		}
	};

	let reply = serde_json::to_vec(&reply).unwrap();
	stream.write_all(&reply).unwrap();
}

impl Drop for MaySocket {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(SOCKET_PATH);
	}
}
