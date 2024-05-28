use super::{Event, Info};
use crate::State;
use smithay::reexports::calloop::{generic::Generic, Interest, LoopHandle, Mode, PostAction};
use std::{
	io::{ErrorKind, Read, Write},
	os::unix::net::{UnixListener, UnixStream},
	path::PathBuf,
	process,
};

#[derive(Debug)]
pub struct MaySocket {
	socket_path: PathBuf,
}

impl MaySocket {
	pub fn init(loop_handle: &LoopHandle<'static, State>, wayland_socket_name: &str) -> MaySocket {
		let socket_name = format!("mayland.{}.{}.sock", wayland_socket_name, process::id());
		let mut socket_path = std::env::temp_dir();
		socket_path.push(socket_name);

		std::env::set_var("MAYLAND_SOCKET", &socket_path);

		let listener = UnixListener::bind(&socket_path).unwrap();
		listener.set_nonblocking(true).unwrap();

		let source = Generic::new(listener, Interest::READ, Mode::Level);
		loop_handle
			.insert_source(source, |_, socket, state| {
				match socket.accept() {
					Ok((stream, _)) => state.on_socket_accept(stream),
					Err(e) if e.kind() == ErrorKind::WouldBlock => (),
					Err(e) => return Err(e),
				}

				Ok(PostAction::Continue)
			})
			.unwrap();

		MaySocket { socket_path }
	}
}

impl Drop for MaySocket {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.socket_path);
	}
}

impl State {
	fn on_socket_accept(&mut self, mut stream: UnixStream) {
		let mut buf = Vec::new();
		stream.read_to_end(&mut buf).unwrap();
		let event = postcard::from_bytes::<Event>(&buf).unwrap();

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
