use mayland::State;
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};

mod trace;

fn main() {
	trace::setup();
	std::panic::set_hook(Box::new(tracing_panic::panic_hook));

	let mut event_loop = EventLoop::<State>::try_new().unwrap();
	let display = Display::<State>::new().unwrap();

	let has_display = std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();
	let mut state = if has_display {
		State::new_winit(&mut event_loop, display)
	} else {
		State::new_udev(&mut event_loop, display)
	};

	// todo
	let xkb = state.mayland.keyboard.clone();
	let keymap = std::fs::read_to_string("/home/may/.config/keymap/comp.xkb").unwrap();
	xkb.set_keymap_from_string(&mut state, keymap).unwrap();

	std::env::set_var("WAYLAND_DISPLAY", &state.mayland.socket_name);
	std::env::set_var("GDK_BACKEND", "wayland");

	event_loop
		.run(None, &mut state, |state| {
			state.mayland.workspaces.refresh();
			state.mayland.popups.cleanup();
			state.mayland.display_handle.flush_clients().unwrap();
		})
		.unwrap();
}
