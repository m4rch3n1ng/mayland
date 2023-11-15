use smithay::desktop::{PopupManager, Space, Window};
use smithay::reexports::calloop::{EventLoop, LoopSignal};
use smithay::wayland::selection::SelectionHandler;
use smithay::{delegate_output, delegate_seat};
use smithay::{
	input::{Seat, SeatHandler, SeatState},
	reexports::wayland_server::{protocol::wl_surface::WlSurface, Display, DisplayHandle},
};
use std::time::Instant;

use crate::Data;

#[derive(Debug)]
pub struct State {
	pub start_time: std::time::Instant,
	display_handle: DisplayHandle,
	seat_state: SeatState<Self>,
	pub popups: PopupManager,
	pub space: Space<Window>,
	pub loop_signal: LoopSignal,
	pub seat: Seat<Self>,
}

impl SeatHandler for State {
	type KeyboardFocus = WlSurface;
	type PointerFocus = WlSurface;

	fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
		&mut self.seat_state
	}
}

impl State {
	pub fn new(event_loop: &mut EventLoop<Data>, display: Display<Self>) -> Self {
		let display_handle = display.handle();
		let mut seat_state = SeatState::new();
		let seat = seat_state.new_wl_seat(&display_handle, "winit");
		let space = Space::default();
		let popups = PopupManager::default();
		let loop_signal = event_loop.get_signal();

		State {
			start_time: Instant::now(),
			popups,
			display_handle,
			seat_state,
			space,
			loop_signal,
			seat,
		}
	}
}

delegate_seat!(State);

impl SelectionHandler for State {
	type SelectionUserData = ();
}

delegate_output!(State);
