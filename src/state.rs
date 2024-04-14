use crate::shell::element::WindowElement;
use smithay::{
	desktop::{PopupManager, Space},
	input::{
		keyboard::{KeyboardHandle, XkbConfig},
		pointer::PointerHandle,
		Seat, SeatState,
	},
	reexports::{
		calloop::{generic::Generic, EventLoop, Interest, LoopSignal, Mode, PostAction},
		wayland_server::{backend::ClientData, Display, DisplayHandle},
	},
	wayland::{
		compositor::{CompositorClientState, CompositorState},
		dmabuf::DmabufState,
		output::OutputManagerState,
		selection::{
			data_device::DataDeviceState, primary_selection::PrimarySelectionState,
			wlr_data_control::DataControlState,
		},
		shell::{wlr_layer::WlrLayerShellState, xdg::XdgShellState},
		shm::ShmState,
		socket::ListeningSocketSource,
	},
};
use std::{sync::Arc, time::Instant};

mod handlers;

#[derive(Debug)]
pub struct State {
	pub display_handle: DisplayHandle,
	pub socket_name: String,

	pub seat: Seat<Self>,
	pub popups: PopupManager,
	pub space: Space<WindowElement>,

	pub start_time: std::time::Instant,
	pub loop_signal: LoopSignal,

	// wayland state
	pub compositor_state: CompositorState,
	pub data_device_state: DataDeviceState,
	pub dmabuf_state: DmabufState,
	pub layer_shell_state: WlrLayerShellState,
	pub output_manager_state: OutputManagerState,
	pub primary_selection_state: PrimarySelectionState,
	pub data_control_state: DataControlState,
	pub seat_state: SeatState<Self>,
	pub xdg_shell_state: XdgShellState,
	pub shm_state: ShmState,

	// input
	pub pointer: PointerHandle<State>,
	pub keyboard: KeyboardHandle<State>,
}

impl State {
	pub fn new(event_loop: &mut EventLoop<Self>, display: Display<Self>) -> Self {
		let display_handle = display.handle();
		let socket_name = init_wayland_display(display, event_loop);

		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "winit");

		let keyboard = seat.add_keyboard(XkbConfig::default(), 200, 25).unwrap();
		let pointer = seat.add_pointer();

		let popups = PopupManager::default();
		let space = Space::default();

		let start_time = Instant::now();
		let loop_signal = event_loop.get_signal();

		let compositor_state = CompositorState::new::<Self>(&display_handle);
		let data_device_state = DataDeviceState::new::<Self>(&display_handle);
		let dmabuf_state = DmabufState::new();
		let layer_shell_state = WlrLayerShellState::new::<Self>(&display_handle);
		let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
		let primary_selection_state = PrimarySelectionState::new::<Self>(&display_handle);
		let data_control_state = DataControlState::new::<Self, _>(
			&display_handle,
			Some(&primary_selection_state),
			|_| true,
		);
		let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
		let shm_state = ShmState::new::<Self>(&display_handle, vec![]);

		State {
			display_handle,
			socket_name,

			seat,
			popups,
			space,

			start_time,
			loop_signal,

			compositor_state,
			data_device_state,
			dmabuf_state,
			layer_shell_state,
			output_manager_state,
			primary_selection_state,
			data_control_state,
			seat_state,
			xdg_shell_state,
			shm_state,

			pointer,
			keyboard,
		}
	}
}

fn init_wayland_display(display: Display<State>, event_loop: &mut EventLoop<State>) -> String {
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
