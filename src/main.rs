use self::state::State;
use clap::{CommandFactory, Parser};
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use std::path::PathBuf;

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

#[derive(Debug, Parser)]
#[clap(version, about)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
#[clap(disable_help_subcommand = true)]
pub struct Args {
	/// a path to a config file
	#[arg(short, long)]
	config: Option<PathBuf>,

	/// print help
	#[arg(long, short, action = clap::ArgAction::Help, global = true)]
	help: Option<bool>,
	/// print version
	#[arg(long, short = 'V', action = clap::ArgAction::Version, global = true)]
	version: Option<bool>,
}

fn main() {
	let args = Args::parse();
	clap_complete::CompleteEnv::with_factory(Args::command).complete();
	trace::setup();

	let mut event_loop = EventLoop::<State>::try_new().unwrap();
	let display = Display::<State>::new().unwrap();

	let mut state = match State::new(&event_loop, display, &args) {
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
