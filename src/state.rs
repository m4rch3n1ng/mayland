use smithay::{
	desktop::{PopupManager, Space, Window},
	input::{keyboard::XkbConfig, Seat, SeatState},
	reexports::{
		calloop::{generic::Generic, EventLoop, Interest, LoopSignal, Mode, PostAction},
		wayland_server::{backend::ClientData, Display, DisplayHandle},
	},
	wayland::{
		compositor::{CompositorClientState, CompositorState},
		selection::data_device::DataDeviceState,
		shell::xdg::XdgShellState,
		shm::ShmState,
		socket::ListeningSocketSource,
	},
};
use std::{sync::Arc, time::Instant};

mod handlers;

#[derive(Debug)]
pub struct MayState {
	pub display_handle: DisplayHandle,
	pub socket_name: String,

	pub seat: Seat<Self>,
	pub popups: PopupManager,
	pub space: Space<Window>,

	pub start_time: std::time::Instant,
	pub loop_signal: LoopSignal,

	// wayland state
	pub compositor_state: CompositorState,
	pub data_device_state: DataDeviceState,
	pub seat_state: SeatState<Self>,
	pub xdg_shell_state: XdgShellState,
	pub shm_state: ShmState,
}

impl MayState {
	pub fn new(event_loop: &mut EventLoop<Self>, display: Display<Self>) -> Self {
		let display_handle = display.handle();
		let socket_name = init_wayland_display(display, event_loop);

		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "winit");

		seat.add_keyboard(XkbConfig::default(), 200, 25).unwrap();
		seat.add_pointer();

		let popups = PopupManager::default();
		let space = Space::default();

		let start_time = Instant::now();
		let loop_signal = event_loop.get_signal();

		let compositor_state = CompositorState::new::<Self>(&display_handle);
		let data_device_state = DataDeviceState::new::<Self>(&display_handle);
		let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
		let shm_state = ShmState::new::<Self>(&display_handle, vec![]);

		MayState {
			display_handle,
			socket_name,

			seat,
			popups,
			space,

			start_time,
			loop_signal,

			compositor_state,
			data_device_state,
			seat_state,
			xdg_shell_state,
			shm_state,
		}
	}
}

fn init_wayland_display(
	display: Display<MayState>,
	event_loop: &mut EventLoop<MayState>,
) -> String {
	// create socket for clients to connect to
	let source = ListeningSocketSource::new_auto().unwrap();
	let socket_name = source.socket_name().to_os_string().into_string().unwrap();

	let handle = event_loop.handle();

	event_loop
		.handle()
		.insert_source(source, move |client_stream, (), state| {
			// insert client into display
			state
				.display_handle
				.insert_client(client_stream, Arc::new(ClientState::default()))
				.unwrap();
		})
		.expect("failed to init the wayland event source.");

	// add display to event loop
	handle
		.insert_source(
			Generic::new(display, Interest::READ, Mode::Level),
			|_, display, state| {
				// SAFETY: we won't drop the display
				unsafe { display.get_mut().dispatch_clients(state).unwrap() };
				Ok(PostAction::Continue)
			},
		)
		.unwrap();

	socket_name
}

#[derive(Default)]
pub struct ClientState {
	pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {}
