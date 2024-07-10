use crate::State;
use smithay::{
	desktop::Window,
	xwayland::{xwm::XwmId, X11Surface, X11Wm, XwmHandler},
};

impl XwmHandler for State {
	fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
		self.mayland.xwm.as_mut().unwrap()
	}

	fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}
	fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

	fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
		window.set_mapped(true).unwrap();
		let window = Window::new_x11_window(window);

		let prev = self.mayland.unmapped_windows.insert(wl_surface, window);
	}
}
