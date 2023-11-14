use clap::{Args, Parser};

#[derive(Debug, Parser)]
#[command(author, version)]
pub struct Cli {
	#[command(flatten)]
	pub init: InitArg,
	#[arg(short, long, value_name = "cmd", help = "startup command")]
	pub exec: Option<String>,
}

impl Cli {
	pub fn init(&self) -> Init {
		let init = &self.init;
		init.to_enum()
	}

	pub fn exec(&self) -> Option<&String> {
		self.exec.as_ref()
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
