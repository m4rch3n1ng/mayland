use crate::{state::Mayland, State};
use calloop::{io::Async, LoopHandle};
use futures_util::{AsyncBufReadExt, AsyncWriteExt};
use mayland_comm::{Request, Response};
use smithay::reexports::calloop::{generic::Generic, Interest, Mode, PostAction};
use std::os::unix::net::{UnixListener, UnixStream};

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

fn handle_stream(state: &mut State, stream: UnixStream) {
	let stream = state.mayland.loop_handle.adapt_io(stream).unwrap();
	let loop_handle = state.mayland.loop_handle.clone();
	let future = async move {
		handle_client(loop_handle, stream).await;
	};

	state.mayland.scheduler.schedule(future).unwrap();
}

async fn handle_client(event_loop: LoopHandle<'static, State>, mut stream: Async<'_, UnixStream>) {
	let mut read = futures_util::io::BufReader::new(&mut stream);
	let mut buf = String::new();
	read.read_line(&mut buf).await.unwrap();

	let request = serde_json::from_str::<Request>(&buf).unwrap();
	let reply = match request {
		Request::Dispatch(action) => {
			let (tx, rx) = async_channel::bounded(1);
			event_loop.insert_idle(move |state| {
				state.handle_action(action);
				let _ = tx.send_blocking(());
			});

			let () = rx.recv().await.unwrap();
			Response::Dispatch
		}
	};

	let reply = serde_json::to_vec(&reply).unwrap();
	stream.write_all(&reply).await.unwrap();
}

impl Drop for MaySocket {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(SOCKET_PATH);
	}
}
