use clap::{Args, Parser};

#[derive(Debug, Parser)]
#[command(author, version)]
pub struct Cli {
	#[command(flatten)]
	pub init: InitArg,
}

impl Cli {
	pub fn init(&self) -> Init {
		let init = &self.init;
		init.to_enum()
	}
}

#[derive(Debug, Args)]
#[group(multiple = false, required = true)]
pub struct InitArg {
	#[arg(long, help = "use winit backend")]
	winit: bool,
	#[arg(long, help = "use from tty")]
	tty: bool,
}

impl InitArg {
	pub fn to_enum(&self) -> Init {
		if self.winit {
			Init::Winit
		} else if self.tty {
			Init::Tty
		} else {
			unreachable!()
		}
	}
}

#[derive(Debug)]
pub enum Init {
	Winit,
	Tty,
}
