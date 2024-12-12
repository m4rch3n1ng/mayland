use super::window::{MappedWindow, UnmappedSurface};
use crate::state::{Mayland, State};
use smithay::{
	backend::renderer::utils::with_renderer_surface_state,
	delegate_presentation, delegate_xdg_shell,
	desktop::PopupKind,
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
		tracing::info!("XdgShellHandler::grab {:?}", surface);
	}

	fn toplevel_destroyed(&mut self, toplevel: ToplevelSurface) {
		if let Some(idx) = self.mayland.unmapped_windows.iter().position(|w| w == &toplevel) {
			let _ = self.mayland.unmapped_windows.remove(idx);
			// an unmapped window got destroyed
			return;
		}

		let Some(window) = self.mayland.workspaces.window_for_surface(&toplevel).cloned() else {
			tracing::error!("couldn't find toplevel");
			return;
		};

		self.mayland.workspaces.remove_window(&window);
		self.reset_focus();
		self.mayland.queue_redraw_all();
	}

	fn popup_destroyed(&mut self, _surface: PopupSurface) {
		self.mayland.queue_redraw_all();
	}

	fn app_id_changed(&mut self, toplevel: ToplevelSurface) {
		if self.mayland.unmapped_windows.iter().any(|w| w == &toplevel) {
			// windowrules are computed when mapping the window
			// so i don't need to deal with unmapped ones
			return;
		}

		let Some(window) = self.mayland.workspaces.window_for_surface(&toplevel) else {
			tracing::error!("couldn't find toplevel");
			return;
		};

		window.recompute_windowrules(&self.mayland.config.windowrules);
	}

	fn title_changed(&mut self, toplevel: ToplevelSurface) {
		if self.mayland.unmapped_windows.iter().any(|w| w == &toplevel) {
			// windowrules are still only computed when mapping
			return;
		}

		let Some(window) = self.mayland.workspaces.window_for_surface(&toplevel) else {
			tracing::error!("couldn't find toplevel");
			return;
		};

		window.recompute_windowrules(&self.mayland.config.windowrules);
	}
}

delegate_xdg_shell!(State);
delegate_presentation!(State);

fn initial_configure_sent(toplevel: &ToplevelSurface) -> bool {
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

impl State {
	/// handle unmapped windows
	pub fn try_map_window(&mut self, surface: &WlSurface) {
		if let Some((idx, unmapped)) = self
			.mayland
			.unmapped_windows
			.iter()
			.enumerate()
			.find(|(_, w)| w == &surface)
		{
			if let Some(toplevel) = unmapped.toplevel() {
				let is_mapped =
					with_renderer_surface_state(surface, |state| state.buffer().is_some()).unwrap_or(false);

				if is_mapped {
					let unmapped = self.mayland.unmapped_windows.remove(idx);

					let windowrules = unmapped.compute_windowrules(&self.mayland.config.windowrules);
					let mapped = MappedWindow::new(unmapped, windowrules);

					mapped.on_commit();

					// add window to workspace
					let location = self.mayland.pointer.current_location();
					self.mayland.workspaces.add_window(mapped.clone(), location);

					// set the window state to be tiled, so that
					// gtk apps don't round their corners
					mapped.set_tiled();

					// automatically focus new windows
					self.focus_window(mapped);

					return;
				}

				if !initial_configure_sent(toplevel) {
					toplevel.send_configure();
				}
			}
		}
	}
}

impl Mayland {
	/// should be called on `WlSurface::commit`
	pub fn handle_surface_commit(&mut self, surface: &WlSurface) {
		// handle toplevel commits
		if let Some(window) = self.workspaces.window_for_surface(surface) {
			window.on_commit();

			if let Some(toplevel) = window.toplevel() {
				if !initial_configure_sent(toplevel) {
					toplevel.send_configure();
				}
			}

			self.handle_resize(window.clone());
			self.queue_redraw_all();
		}

		// handle popup commits.
		self.popups.commit(surface);
		if let Some(PopupKind::Xdg(ref xdg)) = self.popups.find_popup(surface) {
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
	}
}
