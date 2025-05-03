use crate::State;
use calloop::{LoopHandle, io::Async};
use futures_util::{AsyncBufReadExt, AsyncWriteExt};
use mayland_comm::{Request, Response};
use mayland_config::{Action, CONFIG_PATH, bind::CompMod};
use smithay::reexports::calloop::{Interest, Mode, PostAction, generic::Generic};
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
		if let Err(err) = handle_client(stream, socket_state).await {
			tracing::warn!("error handling socket client: {}", err);
		}
	};

	state.mayland.scheduler.schedule(future).unwrap();
}

struct SocketState {
	event_loop: LoopHandle<'static, State>,
	comp_mod: CompMod,
}

async fn handle_client(mut stream: Async<'_, UnixStream>, state: SocketState) -> Result<(), std::io::Error> {
	let mut read = futures_util::io::BufReader::new(&mut stream);
	let mut buf = String::new();
	read.read_line(&mut buf).await?;

	let request = serde_json::from_str::<Request>(&buf);
	let response = match request {
		Ok(Request::Dispatch(action)) => {
			let action = Action::from(action);
			let (tx, rx) = async_channel::bounded(1);
			state.event_loop.insert_idle(move |state| {
				let ret = state.handle_action(action);
				let _ = tx.send_blocking(ret);
			});

			let ret = rx.recv().await.unwrap();
			match ret {
				Ok(()) => Response::Dispatch,
				Err(err) => Response::Err(err),
			}
		}
		Ok(Request::Reload) => 'reload: {
			let Ok(config) = mayland_config::Config::read(&CONFIG_PATH, state.comp_mod) else {
				break 'reload Response::Err(mayland_comm::Error::FailedToReadConfig(CONFIG_PATH.clone()));
			};

			let (tx, rx) = async_channel::bounded(1);
			state.event_loop.insert_idle(move |state| {
				state.reload_config(config);
				let _ = tx.send_blocking(());
			});

			let () = rx.recv().await.unwrap();
			Response::Reload
		}
		Ok(Request::Devices) => {
			let (tx, rx) = async_channel::bounded(1);
			state.event_loop.insert_idle(move |state| {
				let devices = state
					.mayland
					.devices
					.iter()
					.map(mayland_comm::Device::from)
					.collect();

				let _ = tx.send_blocking(devices);
			});

			let devices = rx.recv().await.unwrap();
			Response::Devices(devices)
		}
		Ok(Request::Outputs) => {
			let (tx, rx) = async_channel::bounded(1);
			state.event_loop.insert_idle(move |state| {
				let outputs = state.backend.comm_outputs(&state.mayland);
				let _ = tx.send_blocking(outputs);
			});

			let outputs = rx.recv().await.unwrap();
			Response::Outputs(outputs)
		}
		Ok(Request::Windows) => {
			let (tx, rx) = async_channel::bounded(1);
			state.event_loop.insert_idle(move |state| {
				let keyboard_focus = state.mayland.keyboard.current_focus();
				let keyboard_focus = keyboard_focus.as_ref();

				let windows = state
					.mayland
					.workspaces
					.workspaces
					.values()
					.flat_map(|workspace| {
						workspace
							.windows_geometry()
							.map(move |(window, geometry)| (window, geometry, workspace))
					})
					.map(|(window, geometry, workspace)| {
						window.comm_info(geometry, workspace, keyboard_focus)
					})
					.collect();

				let _ = tx.send_blocking(windows);
			});

			let windows = rx.recv().await.unwrap();
			Response::Windows(windows)
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
	stream.write_all(&response).await?;
	stream.write_all(b"\n").await?;

	Ok(())
}

impl Drop for MaySocket {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.path);
	}
}
