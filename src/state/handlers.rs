use super::State;
use crate::shell::focus::{KeyboardFocusTarget, PointerFocusTarget};
use smithay::{
	backend::{allocator::dmabuf::Dmabuf, input::TabletToolDescriptor},
	delegate_cursor_shape, delegate_data_control, delegate_data_device, delegate_dmabuf, delegate_output,
	delegate_primary_selection, delegate_seat, delegate_viewporter, delegate_xdg_decoration,
	input::{pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
	reexports::{
		wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode,
		wayland_server::{protocol::wl_surface::WlSurface, Resource},
	},
	wayland::{
		dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
		output::OutputHandler,
		seat::WaylandFocus,
		selection::{
			data_device::{
				set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
				ServerDndGrabHandler,
			},
			primary_selection::{set_primary_focus, PrimarySelectionHandler, PrimarySelectionState},
			wlr_data_control::{DataControlHandler, DataControlState},
			SelectionHandler,
		},
		shell::xdg::{decoration::XdgDecorationHandler, ToplevelSurface},
		tablet_manager::TabletSeatHandler,
	},
};

impl SeatHandler for State {
	type KeyboardFocus = KeyboardFocusTarget;
	type PointerFocus = PointerFocusTarget;
	type TouchFocus = WlSurface;

	fn seat_state(&mut self) -> &mut SeatState<Self> {
		&mut self.mayland.seat_state
	}

	fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
		self.mayland.cursor.status = image;
		self.mayland.queue_redraw_all();
	}

	fn focus_changed(&mut self, seat: &Seat<Self>, target: Option<&Self::KeyboardFocus>) {
		let dh = &self.mayland.display_handle;

		let wl_surface = target.and_then(WaylandFocus::wl_surface);
		let client = wl_surface.and_then(|s| dh.get_client(s.id()).ok());
		set_data_device_focus(dh, seat, client.clone());
		set_primary_focus(dh, seat, client);
	}
}

delegate_seat!(State);
delegate_cursor_shape!(State);

impl TabletSeatHandler for State {
	fn tablet_tool_image(&mut self, _tool: &TabletToolDescriptor, image: CursorImageStatus) {
		// todo tablet tools should have their own cursors
		self.mayland.cursor.status = image;
		self.mayland.queue_redraw_all();
	}
}

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

	fn dmabuf_imported(&mut self, _global: &DmabufGlobal, dmabuf: Dmabuf, notifier: ImportNotifier) {
		if self.backend.import_dmabuf(&dmabuf) {
			let _ = notifier.successful::<State>();
		} else {
			notifier.failed();
		}
	}
}

delegate_dmabuf!(State);

impl XdgDecorationHandler for State {
	fn new_decoration(&mut self, toplevel: ToplevelSurface) {
		toplevel.with_pending_state(|state| {
			state.decoration_mode = Some(DecorationMode::ServerSide);
		});
		toplevel.send_pending_configure();
	}

	fn request_mode(&mut self, toplevel: ToplevelSurface, mode: DecorationMode) {
		toplevel.with_pending_state(|state| state.decoration_mode = Some(mode));
		toplevel.send_pending_configure();
	}

	fn unset_mode(&mut self, toplevel: ToplevelSurface) {
		toplevel.with_pending_state(|state| {
			state.decoration_mode = Some(DecorationMode::ServerSide);
		});
		toplevel.send_pending_configure();
	}
}

delegate_xdg_decoration!(State);

delegate_viewporter!(State);
