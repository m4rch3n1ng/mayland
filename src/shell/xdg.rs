use crate::{
	shell::window::UnmappedSurface,
	state::{Mayland, State},
};
use smithay::{
	delegate_layer_shell, delegate_xdg_shell,
	desktop::PopupKind,
	reexports::wayland_server::protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
	utils::Serial,
	wayland::{
		compositor::with_states,
		seat::WaylandFocus,
		shell::xdg::{
			PopupSurface, PositionerState, ToplevelSurface, XdgPopupSurfaceData, XdgShellHandler,
			XdgShellState, XdgToplevelSurfaceData,
		},
	},
};
use tracing::info;

impl XdgShellHandler for State {
	fn xdg_shell_state(&mut self) -> &mut XdgShellState {
		&mut self.mayland.xdg_shell_state
	}

	fn new_toplevel(&mut self, surface: ToplevelSurface) {
		assert!(!self.mayland.unmapped_windows.iter().any(|w| w == &surface));

		let surface = UnmappedSurface::from(surface);
		self.mayland.unmapped_windows.push(surface);
	}

	fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
		let _ = self.mayland.popups.track_popup(PopupKind::Xdg(surface));
	}

	fn reposition_request(&mut self, surface: PopupSurface, positioner: PositionerState, token: u32) {
		surface.with_pending_state(|state| {
			let geometry = positioner.get_geometry();
			state.geometry = geometry;
			state.positioner = positioner;
		});

		surface.send_repositioned(token);
	}

	fn grab(&mut self, surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
		// todo
		info!("XdgShellHandler::grab {:?}", surface);
	}

	fn toplevel_destroyed(&mut self, toplevel: ToplevelSurface) {
		if let Some(idx) = self.mayland.unmapped_windows.iter().position(|w| w == &toplevel) {
			let _ = self.mayland.unmapped_windows.remove(idx);
			// an unmapped window got destroyed
			return;
		}

		let surface = toplevel.wl_surface();
		let window = self.mayland.workspaces.window_for_surface(surface).cloned();
		let Some(window) = window else {
			tracing::error!("couldn't find toplevel");
			return;
		};

		self.mayland.workspaces.remove_window(&window);
		self.reset_keyboard_focus();
		self.mayland.queue_redraw_all();
	}

	fn popup_destroyed(&mut self, _surface: PopupSurface) {
		self.mayland.queue_redraw_all();
	}
}

delegate_xdg_shell!(State);
delegate_layer_shell!(State);

pub fn initial_configure_sent(toplevel: &ToplevelSurface) -> bool {
	with_states(toplevel.wl_surface(), |states| {
		states
			.data_map
			.get::<XdgToplevelSurfaceData>()
			.unwrap()
			.lock()
			.unwrap()
			.initial_configure_sent
	})
}

impl Mayland {
	/// should be called on `WlSurface::commit`
	pub fn handle_surface_commit(&mut self, surface: &WlSurface) {
		// handle toplevel commits.
		if let Some(mapped) = self
			.workspaces
			.windows()
			.find(|w| w.wl_surface().is_some_and(|w| *w == *surface))
			.cloned()
		{
			if let Some(toplevel) = mapped.window.toplevel() {
				if !initial_configure_sent(toplevel) {
					toplevel.send_configure();
				}
			}
		}

		// handle popup commits.
		self.popups.commit(surface);
		if let Some(popup) = self.popups.find_popup(surface) {
			match popup {
				PopupKind::Xdg(ref xdg) => {
					let initial_configure_sent = with_states(surface, |states| {
						states
							.data_map
							.get::<XdgPopupSurfaceData>()
							.unwrap()
							.lock()
							.unwrap()
							.initial_configure_sent
					});
					if !initial_configure_sent {
						// NOTE: this should never fail as the initial configure is always allowed.
						xdg.send_configure().expect("initial configure failed");
					}
				}
				PopupKind::InputMethod(ref _input_method) => {}
			}
		}
	}
}
