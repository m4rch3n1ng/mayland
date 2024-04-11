use super::element::WindowElement;
use crate::state::State;
use smithay::{
	delegate_layer_shell, delegate_xdg_shell,
	desktop::{PopupKind, PopupManager, Space, Window},
	reexports::wayland_server::protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
	utils::Serial,
	wayland::{
		compositor::with_states,
		shell::xdg::{
			PopupSurface, PositionerState, ToplevelSurface, XdgPopupSurfaceData, XdgShellHandler,
			XdgShellState, XdgToplevelSurfaceData,
		},
	},
};

impl XdgShellHandler for State {
	fn xdg_shell_state(&mut self) -> &mut XdgShellState {
		&mut self.xdg_shell_state
	}

	fn new_toplevel(&mut self, surface: ToplevelSurface) {
		let window = WindowElement(Window::new_wayland_window(surface));
		self.space.map_element(window, (0, 0), false);
	}

	fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
		let _ = self.popups.track_popup(PopupKind::Xdg(surface));
	}

	fn reposition_request(
		&mut self,
		surface: PopupSurface,
		positioner: PositionerState,
		token: u32,
	) {
		surface.with_pending_state(|state| {
			let geometry = positioner.get_geometry();
			state.geometry = geometry;
			state.positioner = positioner;
		});

		surface.send_repositioned(token);
	}

	fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
		// todo
	}
}

delegate_xdg_shell!(State);
delegate_layer_shell!(State);

/// Should be called on `WlSurface::commit`
pub fn handle_commit(popups: &mut PopupManager, space: &Space<WindowElement>, surface: &WlSurface) {
	// Handle toplevel commits.
	if let Some(window) = space
		.elements()
		.find(|w| w.wl_surface().is_some_and(|w| w == *surface))
		.cloned()
	{
		if let Some(toplevel) = window.0.toplevel() {
			let initial_configure_sent = with_states(surface, |states| {
				states
					.data_map
					.get::<XdgToplevelSurfaceData>()
					.unwrap()
					.lock()
					.unwrap()
					.initial_configure_sent
			});

			if !initial_configure_sent {
				toplevel.send_configure();
			}
		}
	}

	// Handle popup commits.
	popups.commit(surface);
	if let Some(popup) = popups.find_popup(surface) {
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
					// NOTE: This should never fail as the initial configure is always
					// allowed.
					xdg.send_configure().expect("initial configure failed");
				}
			}
			PopupKind::InputMethod(ref _input_method) => {}
		}
	}
}
