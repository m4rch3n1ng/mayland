use smithay::desktop::{PopupManager, Space, Window};
use smithay::input::keyboard::XkbConfig;
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{EventLoop, Interest, LoopSignal, PostAction};
use smithay::reexports::wayland_server::backend::ClientData;
use smithay::wayland::compositor::{CompositorClientState, CompositorState};
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shm::ShmState;
use smithay::wayland::socket::ListeningSocketSource;
use smithay::{
	input::{Seat, SeatState},
	reexports::{
		calloop::Mode,
		wayland_server::{Display, DisplayHandle},
	},
};
use std::ffi::OsString;
use std::sync::Arc;
use std::time::Instant;

mod compositor;
mod handlers;
mod xdg;

#[derive(Debug)]
pub struct MayState {
	pub start_time: std::time::Instant,
	pub display_handle: DisplayHandle,
	pub seat_state: SeatState<Self>,
	pub data_device_state: DataDeviceState,
	pub popups: PopupManager,
	pub space: Space<Window>,
	pub loop_signal: LoopSignal,
	pub seat: Seat<Self>,
	pub xdg_shell_state: XdgShellState,
	pub socket_name: String,
	pub shm_state: ShmState,
	pub compositor_state: CompositorState,
}

impl MayState {
	pub fn new(event_loop: &mut EventLoop<MayState>, display: Display<Self>) -> Self {
		let display_handle = display.handle();
		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "winit");
		let space = Space::default();
		let popups = PopupManager::default();
		let loop_signal = event_loop.get_signal();
		let data_device_state = DataDeviceState::new::<Self>(&display_handle);
		let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);

		let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
		let compositor_state = CompositorState::new::<Self>(&display_handle);

		seat.add_keyboard(XkbConfig::default(), 200, 25).unwrap();
		seat.add_pointer();

		let socket_name = Self::init_wayland_listener(display, event_loop)
			.into_string()
			.unwrap();

		MayState {
			start_time: Instant::now(),
			popups,
			data_device_state,
			display_handle,
			seat_state,
			space,
			socket_name,
			loop_signal,
			seat,
			xdg_shell_state,
			compositor_state,
			shm_state,
		}
	}

	fn init_wayland_listener(
		display: Display<MayState>,
		event_loop: &mut EventLoop<MayState>,
	) -> OsString {
		// Creates a new listening socket, automatically choosing the next available `wayland` socket name.
		let listening_socket = ListeningSocketSource::new_auto().unwrap();

		// Get the name of the listening socket.
		// Clients will connect to this socket.
		let socket_name = listening_socket.socket_name().to_os_string();

		let handle = event_loop.handle();

		event_loop
			.handle()
			.insert_source(listening_socket, move |client_stream, (), state| {
				// Inside the callback, you should insert the client into the display.
				//
				// You may also associate some data with the client when inserting the client.
				state
					.display_handle
					.insert_client(client_stream, Arc::new(ClientState::default()))
					.unwrap();
			})
			.expect("Failed to init the wayland event source.");

		// You also need to add the display itself to the event loop, so that client events will be processed by wayland-server.
		handle
			.insert_source(
				Generic::new(display, Interest::READ, Mode::Level),
				|_, display, state| {
					// Safety: we don't drop the display
					unsafe {
						display.get_mut().dispatch_clients(state).unwrap();
					}
					Ok(PostAction::Continue)
				},
			)
			.unwrap();

		socket_name
	}
}

#[derive(Default)]
pub struct ClientState {
	pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {}
