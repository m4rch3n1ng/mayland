use crate::cli::{Cli, Init};
use clap::Parser;
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use state::State;
use std::process::Command;

mod action;
mod backend;
mod cli;
mod input;
mod shell;
mod state;

fn main() {
	let args = Cli::parse();

	let init = args.init();
	let exec = args.exec();

	let mut event_loop = EventLoop::<State>::try_new().unwrap();

	let display = Display::<State>::new().unwrap();
	let mut state = match init {
		Init::Winit => State::new_winit(&mut event_loop, display),
		Init::Tty => todo!("tty"),
	};

	// todo
	let xkb = state.mayland.keyboard.clone();
	let keymap = std::fs::read_to_string("/home/may/.config/keymap/comp.xkb").unwrap();
	xkb.set_keymap_from_string(&mut state, keymap).unwrap();

	std::env::set_var("WAYLAND_DISPLAY", &state.mayland.socket_name);
	std::env::set_var("GDK_BACKEND", "wayland");

	if let Some(exec) = exec {
		let split = exec.split_whitespace().collect::<Vec<_>>();
		let [cmd, args @ ..] = &split[..] else {
			panic!()
		};

		println!("exec {:?} {:?}", cmd, args);

		Command::new(cmd).args(args).spawn().unwrap();
	}

	event_loop.run(None, &mut state, |_| {}).unwrap();
}
