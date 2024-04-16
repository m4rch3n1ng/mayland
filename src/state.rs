use crate::{
	backend::{Backend, Winit},
	shell::element::WindowElement,
};
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
		shell::{
			wlr_layer::WlrLayerShellState,
			xdg::{decoration::XdgDecorationState, XdgShellState},
		},
		shm::ShmState,
		socket::ListeningSocketSource,
	},
};
use std::{collections::HashSet, sync::Arc, time::Instant};

mod handlers;

pub struct State {
	pub backend: Backend,
	pub mayland: Mayland,
}

impl State {
	pub fn new_winit(event_loop: &mut EventLoop<State>, display: Display<State>) -> Self {
		let mut space = Space::default();

		let winit = Winit::init(event_loop, &mut display.handle(), &mut space);
		let winit = Backend::Winit(winit);

		let mayland = Mayland::new(event_loop, display, space);

		State {
			backend: winit,
			mayland,
		}
	}
}

#[derive(Debug)]
pub struct Mayland {
	pub display_handle: DisplayHandle,
	pub socket_name: String,

	pub seat: Seat<State>,
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
	pub seat_state: SeatState<State>,
	pub xdg_decoration_state: XdgDecorationState,
	pub xdg_shell_state: XdgShellState,
	pub shm_state: ShmState,

	// input
	pub pointer: PointerHandle<State>,
	pub keyboard: KeyboardHandle<State>,

	pub suppressed_keys: HashSet<u32>,
}

impl Mayland {
	fn new(
		event_loop: &mut EventLoop<State>,
		display: Display<State>,
		space: Space<WindowElement>,
	) -> Self {
		let display_handle = display.handle();
		let socket_name = init_wayland_display(display, event_loop);

		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "winit");

		let popups = PopupManager::default();

		let start_time = Instant::now();
		let loop_signal = event_loop.get_signal();

		let compositor_state = CompositorState::new::<State>(&display_handle);
		let data_device_state = DataDeviceState::new::<State>(&display_handle);
		let dmabuf_state = DmabufState::new();
		let layer_shell_state = WlrLayerShellState::new::<State>(&display_handle);
		let output_manager_state =
			OutputManagerState::new_with_xdg_output::<State>(&display_handle);
		let primary_selection_state = PrimarySelectionState::new::<State>(&display_handle);
		let data_control_state = DataControlState::new::<State, _>(
			&display_handle,
			Some(&primary_selection_state),
			|_| true,
		);
		let xdg_decoration_state = XdgDecorationState::new::<State>(&display_handle);
		let xdg_shell_state = XdgShellState::new::<State>(&display_handle);
		let shm_state = ShmState::new::<State>(&display_handle, vec![]);

		let keyboard = seat.add_keyboard(XkbConfig::default(), 200, 25).unwrap();
		let pointer = seat.add_pointer();

		let suppressed_keys = HashSet::new();

		Mayland {
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
			xdg_decoration_state,
			xdg_shell_state,
			shm_state,

			pointer,
			keyboard,

			suppressed_keys,
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
				.mayland
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
