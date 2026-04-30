use super::State;
use crate::shell::focus::{KeyboardFocusTarget, PointerFocusTarget};
use smithay::{
	backend::{allocator::dmabuf::Dmabuf, input::TabletToolDescriptor},
	input::{
		Seat, SeatHandler, SeatState,
		dnd::{self, DnDGrab, DndGrabHandler, GrabType},
		pointer::{CursorImageStatus, Focus},
	},
	reexports::{
		wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode,
		wayland_server::{Resource, protocol::wl_surface::WlSurface},
	},
	wayland::{
		dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
		output::OutputHandler,
		seat::WaylandFocus,
		selection::{
			SelectionHandler,
			data_device::{DataDeviceHandler, DataDeviceState, WaylandDndGrabHandler, set_data_device_focus},
			primary_selection::{PrimarySelectionHandler, PrimarySelectionState, set_primary_focus},
			wlr_data_control::{DataControlHandler, DataControlState},
		},
		shell::xdg::{ToplevelSurface, decoration::XdgDecorationHandler},
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
	fn data_device_state(&mut self) -> &mut DataDeviceState {
		&mut self.mayland.data_device_state
	}
}

impl DndGrabHandler for State {}
impl WaylandDndGrabHandler for State {
	fn dnd_requested<S: dnd::Source>(
		&mut self,
		source: S,
		_icon: Option<WlSurface>,
		seat: Seat<Self>,
		serial: smithay::utils::Serial,
		r#type: smithay::input::dnd::GrabType,
	) {
		match r#type {
			GrabType::Pointer => {
				let ptr = seat.get_pointer().unwrap();
				let start_data = ptr.grab_start_data().unwrap();

				let grab = DnDGrab::new_pointer(&self.mayland.display_handle, start_data, source, seat);
				ptr.set_grab(self, grab, serial, Focus::Keep);
			}
			// TODO: touch handling
			GrabType::Touch => {}
		}
	}
}

impl OutputHandler for State {}

impl PrimarySelectionHandler for State {
	fn primary_selection_state(&mut self) -> &mut PrimarySelectionState {
		&mut self.mayland.primary_selection_state
	}
}

impl DataControlHandler for State {
	fn data_control_state(&mut self) -> &mut DataControlState {
		&mut self.mayland.data_control_state
	}
}

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
