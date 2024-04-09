use std::process::Command;

use crate::cli::{Cli, Init};
use clap::Parser;
use smithay::reexports::{
	calloop::EventLoop,
	wayland_server::{Display, DisplayHandle},
};
use state::MayState;

mod cli;
mod state;
mod winit;

#[derive(Debug)]
pub struct Data {
	state: MayState,
	display_handle: DisplayHandle,
}

fn main() {
	let args = Cli::parse();

	let init = args.init();
	let exec = args.exec();

	match init {
		Init::Winit => println!("winit"),
		Init::Tty => todo!("tty"),
	}

	let mut event_loop = EventLoop::<Data>::try_new().unwrap();

	let display = Display::<MayState>::new().unwrap();
	let display_handle = display.handle();

	let state = MayState::new(&mut event_loop, display);
	let mut data = Data {
		state,
		display_handle,
	};

	winit::init(&mut event_loop, &mut data);

	println!("exec {:?}", exec);
	if let Some(cmd) = exec {
		Command::new(cmd).spawn().unwrap();
	}

	event_loop
		.run(None, &mut data, move |_| {
			// println!("data {:?}", ev);
		})
		.unwrap();
}
