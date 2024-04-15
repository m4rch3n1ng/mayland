use super::State;
use crate::shell::focus::{KeyboardFocusTarget, PointerFocusTarget};
use smithay::{
	backend::allocator::dmabuf::Dmabuf,
	delegate_data_control, delegate_data_device, delegate_dmabuf, delegate_output,
	delegate_primary_selection, delegate_seat,
	input::SeatHandler,
	reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
	wayland::{
		dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
		output::OutputHandler,
		seat::WaylandFocus,
		selection::{
			data_device::{
				set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
				ServerDndGrabHandler,
			},
			primary_selection::{PrimarySelectionHandler, PrimarySelectionState},
			wlr_data_control::{DataControlHandler, DataControlState},
			SelectionHandler,
		},
	},
};

impl SeatHandler for State {
	type KeyboardFocus = KeyboardFocusTarget;
	type PointerFocus = PointerFocusTarget;
	type TouchFocus = WlSurface;

	fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
		&mut self.mayland.seat_state
	}

	fn focus_changed(
		&mut self,
		seat: &smithay::input::Seat<Self>,
		target: Option<&Self::KeyboardFocus>,
	) {
		let dh = &self.mayland.display_handle;

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
		&self.mayland.data_device_state
	}
}

impl ClientDndGrabHandler for State {}
impl ServerDndGrabHandler for State {}

delegate_data_device!(State);

impl OutputHandler for State {}

delegate_output!(State);

impl PrimarySelectionHandler for State {
	fn primary_selection_state(&self) -> &PrimarySelectionState {
		&self.mayland.primary_selection_state
	}
}

delegate_primary_selection!(State);

impl DataControlHandler for State {
	fn data_control_state(&self) -> &DataControlState {
		&self.mayland.data_control_state
	}
}

delegate_data_control!(State);

impl DmabufHandler for State {
	fn dmabuf_state(&mut self) -> &mut DmabufState {
		&mut self.mayland.dmabuf_state
	}

	fn dmabuf_imported(
		&mut self,
		_global: &DmabufGlobal,
		_dmabuf: Dmabuf,
		_notifier: ImportNotifier,
	) {
		todo!();
	}
}

delegate_dmabuf!(State);
