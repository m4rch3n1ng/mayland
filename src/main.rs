use crate::cli::{Cli, Init};
use clap::Parser;

mod cli;

fn main() {
	let args = Cli::parse();

	let init = args.init();
	let exec = args.exec();

	println!("exec {:?}", exec);

	match init {
		Init::Winit => println!("winit"),
		Init::Tty => todo!("tty"),
	}
}
