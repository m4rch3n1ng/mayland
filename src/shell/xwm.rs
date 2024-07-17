use super::window::UnmappedSurface;
use crate::{shell::window::MappedWindow, State};
use smithay::{
	delegate_xwayland_shell,
	reexports::x11rb::protocol::xproto,
	utils::{Logical, Rectangle},
	wayland::xwayland_shell::{XWaylandShellHandler, XWaylandShellState},
	xwayland::{
		xwm::{self, Reorder, XwmId},
		X11Surface, X11Wm, XwmHandler,
	},
};

impl XWaylandShellHandler for State {
	fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
		&mut self.mayland.xwayland_shell_state
	}
}

impl XwmHandler for State {
	fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
		self.mayland.xwm.as_mut().unwrap()
	}

	fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}
	fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

	fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
		window.set_mapped(true).unwrap();

		assert!(!self.mayland.unmapped_windows.iter().any(|w| w == &window));

		let surface = UnmappedSurface::from(window);
		self.mayland.unmapped_windows.push(surface);
	}

	fn map_window_notify(&mut self, _xwm: XwmId, surface: X11Surface) {
		tracing::info!("XwmHandler::map_window_notify");

		if let Some(idx) = self.mayland.unmapped_windows.iter().position(|w| w == &surface) {
			let unmapped = self.mayland.unmapped_windows.remove(idx);
			let windowrules = unmapped.compute_windowrules(&self.mayland.config.windowrules);
			let window = MappedWindow::new(unmapped, windowrules);

			let location = self.mayland.pointer.current_location();
			self.mayland.workspaces.add_window(window.clone(), location);

			self.focus_window(window);
		}
	}

	fn mapped_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {
		tracing::info!("XwmHandler::todo mapped_override_redirect_window");
	}

	fn unmapped_window(&mut self, _xwm: XwmId, xsurface: X11Surface) {
		if let Some(idx) = self.mayland.unmapped_windows.iter().position(|w| w == &xsurface) {
			let _ = self.mayland.unmapped_windows.remove(idx);
			// an unmapped window got destroyed
			return;
		}

		let window = self.mayland.workspaces.window_for_surface(&xsurface).cloned();
		let Some(window) = window else {
			tracing::error!("couldn't find window");
			return;
		};

		self.mayland.workspaces.remove_window(&window);
		self.reset_focus();
		self.mayland.queue_redraw_all();
	}

	fn destroyed_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

	fn configure_request(
		&mut self,
		_xwm: XwmId,
		_window: X11Surface,
		_x: Option<i32>,
		_y: Option<i32>,
		_w: Option<u32>,
		_h: Option<u32>,
		_reorder: Option<Reorder>,
	) {
		// tracing::info!("XwmHandler::configure_request");
	}

	fn configure_notify(
		&mut self,
		_xwm: XwmId,
		_window: X11Surface,
		_geometry: Rectangle<i32, Logical>,
		_above: Option<xproto::Window>,
	) {
		// tracing::info!("XwmHandler::configure_notify");
	}

	fn resize_request(
		&mut self,
		_xwm: XwmId,
		_window: X11Surface,
		_button: u32,
		_resize_edge: xwm::ResizeEdge,
	) {
		// explicity ignore resize_request
	}

	fn move_request(&mut self, _xwm: XwmId, _window: X11Surface, _button: u32) {
		// explicity ignore move_request
	}
}

delegate_xwayland_shell!(State);
