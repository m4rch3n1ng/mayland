use crate::cli::{Cli, Init};
use clap::Parser;
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use state::State;
use std::process::Command;

mod cli;
mod input;
mod shell;
mod state;
mod winit;

fn main() {
	let args = Cli::parse();

	let init = args.init();
	let exec = args.exec();

	let mut event_loop = EventLoop::<State>::try_new().unwrap();

	let display = Display::<State>::new().unwrap();
	let mut state = State::new(&mut event_loop, display);

	// todo
	let xkb = state.seat.get_keyboard().unwrap();
	let keymap = std::fs::read_to_string("/home/may/.config/keymap/comp.xkb").unwrap();
	xkb.set_keymap_from_string(&mut state, keymap).unwrap();

	match init {
		Init::Winit => winit::init(&mut event_loop, &mut state),
		Init::Tty => todo!("tty"),
	}

	if let Some(cmd) = exec {
		println!("exec {:?}", cmd);
		Command::new(cmd)
			.envs([("WAYLAND_DISPLAY", &state.socket_name)])
			.spawn()
			.unwrap();
	}

	event_loop
		.run(None, &mut state, move |_| {
			// println!("data {:?}", ev);
		})
		.unwrap();
}
