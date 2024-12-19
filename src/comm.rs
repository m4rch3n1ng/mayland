use crate::State;
use calloop::{io::Async, LoopHandle};
use futures_util::{AsyncBufReadExt, AsyncWriteExt};
use mayland_comm::{Request, Response};
use smithay::reexports::calloop::{generic::Generic, Interest, Mode, PostAction};
use std::{
	os::unix::net::{UnixListener, UnixStream},
	path::PathBuf,
};

#[derive(Debug)]
pub struct MaySocket {
	pub path: PathBuf,
}

fn socket_path(wayland_socket_name: &str) -> PathBuf {
	let mut runtime_dir = dirs::runtime_dir().unwrap_or_else(std::env::temp_dir);
	let socket_name = format!("mayland.{}.{}.sock", wayland_socket_name, std::process::id());
	runtime_dir.push(socket_name);

	runtime_dir
}

impl MaySocket {
	pub fn init(loop_handle: &LoopHandle<'static, State>, wayland_socket_name: &str) -> Self {
		let socket_path = socket_path(wayland_socket_name);

		let listener = UnixListener::bind(&socket_path).unwrap();
		listener.set_nonblocking(true).unwrap();

		let source = Generic::new(listener, Interest::READ, Mode::Level);
		loop_handle
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

	let request = serde_json::from_str::<Request>(&buf);
	let reply = match request {
		Ok(Request::Dispatch(action)) => {
			let (tx, rx) = async_channel::bounded(1);
			event_loop.insert_idle(move |state| {
				state.handle_action(action);
				let _ = tx.send_blocking(());
			});

			let () = rx.recv().await.unwrap();
			Response::Dispatch
		}
		Ok(Request::Workspaces) => {
			let (tx, rx) = async_channel::bounded(1);
			event_loop.insert_idle(move |state| {
				let workspaces = state
					.mayland
					.workspaces
					.workspaces
					.values()
					.map(|workspace| workspace.comm_info(&state.mayland.workspaces))
					.collect();

				let _ = tx.send_blocking(workspaces);
			});

			let workspaces = rx.recv().await.unwrap();
			Response::Workspaces(workspaces)
		}
		Err(_) => Response::Err(mayland_comm::Error::InvalidRequest),
	};

	let reply = serde_json::to_vec(&reply).unwrap();
	stream.write_all(&reply).await.unwrap();
	stream.write_all(b"\n").await.unwrap();
}

impl Drop for MaySocket {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.path);
	}
}
