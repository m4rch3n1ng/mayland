use crate::cli::{Cli, Init};
use clap::Parser;
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use state::MayState;
use std::process::Command;

mod cli;
mod input;
mod state;
mod winit;

fn main() {
	let args = Cli::parse();

	let init = args.init();
	let exec = args.exec();

	let mut event_loop = EventLoop::<MayState>::try_new().unwrap();

	let display = Display::<MayState>::new().unwrap();
	let mut state = MayState::new(&mut event_loop, display);

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
