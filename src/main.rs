use mayland::{MaylandError, State};
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};

mod trace;

fn main() {
	trace::setup();
	std::panic::set_hook(Box::new(tracing_panic::panic_hook));

	let mut event_loop = EventLoop::<State>::try_new().unwrap();
	let display = Display::<State>::new().unwrap();

	let has_display = std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();
	let state = if has_display {
		State::new_winit(&mut event_loop, display)
	} else {
		State::new_udev(&mut event_loop, display)
	};

	let mut state = match state {
		Ok(state) => Ok(state),
		Err(MaylandError::AlreadyPrinted) => return,
		Err(e) => Err(e),
	}
	.unwrap();
	state.load_config();

	state.mayland.environment.extend([
		("WAYLAND_DISPLAY".to_owned(), state.mayland.socket_name.clone()),
		("GDK_BACKEND".to_owned(), "wayland".to_owned()),
	]);

	event_loop
		.run(None, &mut state, |state| {
			state.mayland.workspaces.refresh();
			state.mayland.popups.cleanup();
			state.mayland.display_handle.flush_clients().unwrap();
		})
		.unwrap();
}
