use crate::State;
use calloop::{io::Async, LoopHandle};
use futures_util::{AsyncBufReadExt, AsyncWriteExt};
use mayland_comm::{Request, Response};
use mayland_config::bind::CompMod;
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
	let socket_state = SocketState {
		event_loop: state.mayland.loop_handle.clone(),
		comp_mod: state.mayland.comp_mod,
	};

	let future = async move {
		handle_client(stream, socket_state).await;
	};

	state.mayland.scheduler.schedule(future).unwrap();
}

struct SocketState {
	event_loop: LoopHandle<'static, State>,
	comp_mod: CompMod,
}

async fn handle_client(mut stream: Async<'_, UnixStream>, state: SocketState) {
	let mut read = futures_util::io::BufReader::new(&mut stream);
	let mut buf = String::new();
	read.read_line(&mut buf).await.unwrap();

	let request = serde_json::from_str::<Request>(&buf);
	let response = match request {
		Ok(Request::Dispatch(action)) => {
			let (tx, rx) = async_channel::bounded(1);
			state.event_loop.insert_idle(move |state| {
				state.handle_action(action);
				let _ = tx.send_blocking(());
			});

			let () = rx.recv().await.unwrap();
			Response::Dispatch
		}
		Ok(Request::Reload) => 'reload: {
			let Ok(config) = mayland_config::Config::read(state.comp_mod) else {
				break 'reload Response::Err(mayland_comm::Error::FailedToReadConfig);
			};

			let (tx, rx) = async_channel::bounded(1);
			state.event_loop.insert_idle(move |state| {
				state.reload_config(config);
				let _ = tx.send_blocking(());
			});

			let () = rx.recv().await.unwrap();
			Response::Reload
		}
		Ok(Request::Outputs) => {
			let (tx, rx) = async_channel::bounded(1);
			state.event_loop.insert_idle(move |state| {
				let outputs = state.backend.comm_outputs();
				let _ = tx.send_blocking(outputs);
			});

			let outputs = rx.recv().await.unwrap();
			Response::Outputs(outputs)
		}
		Ok(Request::Workspaces) => {
			let (tx, rx) = async_channel::bounded(1);
			state.event_loop.insert_idle(move |state| {
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

	let response = serde_json::to_vec(&response).unwrap();
	stream.write_all(&response).await.unwrap();
	stream.write_all(b"\n").await.unwrap();
}

impl Drop for MaySocket {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.path);
	}
}
