use super::MayState;
use smithay::{
	delegate_data_device, delegate_output, delegate_seat,
	input::SeatHandler,
	reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
	wayland::{
		output::OutputHandler,
		selection::{
			data_device::{
				set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
				ServerDndGrabHandler,
			},
			SelectionHandler,
		},
	},
};

impl SeatHandler for MayState {
	type KeyboardFocus = WlSurface;
	type PointerFocus = WlSurface;
	type TouchFocus = WlSurface;

	fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
		&mut self.seat_state
	}

	fn focus_changed(
		&mut self,
		seat: &smithay::input::Seat<Self>,
		focused: Option<&Self::KeyboardFocus>,
	) {
		let dh = &self.display_handle;
		let client = focused.and_then(|s| dh.get_client(s.id()).ok());
		set_data_device_focus(dh, seat, client);
	}
}

delegate_seat!(MayState);

impl SelectionHandler for MayState {
	type SelectionUserData = ();
}

impl DataDeviceHandler for MayState {
	fn data_device_state(&self) -> &DataDeviceState {
		&self.data_device_state
	}
}

impl ClientDndGrabHandler for MayState {}
impl ServerDndGrabHandler for MayState {}

delegate_data_device!(MayState);

impl OutputHandler for MayState {}

delegate_output!(MayState);
