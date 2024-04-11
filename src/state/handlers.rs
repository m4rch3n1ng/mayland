use super::State;
use crate::shell::focus::{KeyboardFocusTarget, PointerFocusTarget};
use smithay::{
	delegate_data_device, delegate_output, delegate_seat,
	input::SeatHandler,
	reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
	wayland::{
		output::OutputHandler,
		seat::WaylandFocus,
		selection::{
			data_device::{
				set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
				ServerDndGrabHandler,
			},
			SelectionHandler,
		},
	},
};

impl SeatHandler for State {
	type KeyboardFocus = KeyboardFocusTarget;
	type PointerFocus = PointerFocusTarget;
	type TouchFocus = WlSurface;

	fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
		&mut self.seat_state
	}

	fn focus_changed(
		&mut self,
		seat: &smithay::input::Seat<Self>,
		target: Option<&Self::KeyboardFocus>,
	) {
		let dh = &self.display_handle;

		let wl_surface = target.and_then(WaylandFocus::wl_surface);
		let client = wl_surface.and_then(|s| dh.get_client(s.id()).ok());
		set_data_device_focus(dh, seat, client);
	}
}

delegate_seat!(State);

impl SelectionHandler for State {
	type SelectionUserData = ();
}

impl DataDeviceHandler for State {
	fn data_device_state(&self) -> &DataDeviceState {
		&self.data_device_state
	}
}

impl ClientDndGrabHandler for State {}
impl ServerDndGrabHandler for State {}

delegate_data_device!(State);

impl OutputHandler for State {}

delegate_output!(State);
