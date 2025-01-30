use self::state::State;
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};

mod backend;
mod comm;
mod cursor;
mod input;
mod layout;
mod render;
mod shell;
mod state;
mod trace;
mod utils;

fn main() {
	trace::setup();

	let mut event_loop = EventLoop::<State>::try_new().unwrap();
	let display = Display::<State>::new().unwrap();

	let has_display = std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();
	let state = if has_display {
		State::new_winit(&mut event_loop, display)
	} else {
		State::new_udev(&mut event_loop, display)
	};

	let mut state = match state {
		Ok(state) => state,
		Err(err) => {
			anstream::println!("{}", err);
			return;
		}
	};

	state.mayland.environment.extend([
		("WAYLAND_DISPLAY".to_owned(), state.mayland.socket_name.clone()),
		("XDG_SESSION_TYPE".to_owned(), "wayland".to_owned()),
		("GDK_BACKEND".to_owned(), "wayland".to_owned()),
	]);

	event_loop
		.run(None, &mut state, |state| {
			state.refresh_and_redraw();
		})
		.unwrap();
}
