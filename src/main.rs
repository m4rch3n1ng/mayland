use std::process::Command;

use crate::cli::{Cli, Init};
use clap::Parser;
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use state::MayState;

mod cli;
mod state;
mod winit;

fn main() {
	let args = Cli::parse();

	let init = args.init();
	let exec = args.exec();

	match init {
		Init::Winit => println!("winit"),
		Init::Tty => todo!("tty"),
	}

	let mut event_loop = EventLoop::<MayState>::try_new().unwrap();

	let display = Display::<MayState>::new().unwrap();
	let mut state = MayState::new(&mut event_loop, display);

	winit::init(&mut event_loop, &mut state);

	println!("exec {:?}", exec);
	if let Some(cmd) = exec {
		Command::new(cmd).spawn().unwrap();
	}

	event_loop
		.run(None, &mut state, move |_| {
			// println!("data {:?}", ev);
		})
		.unwrap();
}
