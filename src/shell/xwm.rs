use super::window::UnmappedSurface;
use crate::State;
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

	fn map_window_notify(&mut self, _xwm: XwmId, _window: X11Surface) {
		// todo map window here
		tracing::info!("XwmHandler::map_window_notify");
	}

	fn mapped_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {
		tracing::info!("XwmHandler::todo mapped_override_redirect_window");
	}

	fn unmapped_window(&mut self, _xwm: XwmId, _window: X11Surface) {
		tracing::info!("XwmHandler::unmapped_window");
	}

	fn destroyed_window(&mut self, _xwm: XwmId, _window: X11Surface) {
		tracing::info!("XwmHandler::destroyed_window");
	}

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
		tracing::info!("XwmHandler::configure_notify");
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
