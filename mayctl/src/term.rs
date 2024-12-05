use mayland_comm::MAYLAND_SOCKET_VAR;
use owo_colors::OwoColorize;
use std::{fmt::Display, io::Write as _, process::Termination};

pub enum Term {
	/// mayctl exited successfully
	Ok,
	/// mayctl wasn't started inside mayland
	MaylandNotRunning,
}

impl Termination for Term {
	fn report(self) -> std::process::ExitCode {
		if let Term::Ok = self {
			std::process::ExitCode::SUCCESS
		} else {
			let mut stderr = anstream::stderr();
			let _ = write!(stderr, "{}", self);

			std::process::ExitCode::FAILURE
		}
	}
}

impl Display for Term {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Term::Ok => Ok(()),
			Term::MaylandNotRunning => {
				writeln!(f, "{}: {}", "error".red().bold(), "mayland not running".bold())?;
				writeln!(f, "  {} env ${} not set", "::".blue().bold(), MAYLAND_SOCKET_VAR)
			}
		}
	}
}
