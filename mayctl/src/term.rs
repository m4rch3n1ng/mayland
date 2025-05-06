use mayland_comm::{MAYLAND_SOCKET_VAR, Response};
use owo_colors::OwoColorize;
use std::{fmt::Display, io::Write as _, path::PathBuf, process::Termination};

pub enum Term {
	/// mayctl exited successfully
	Ok,
	/// an io error occured
	IoError(PathBuf, std::io::Error),
	/// the mayland socket was not found
	NotFound(PathBuf),
	/// mayland returned a response that couldn't be deserialized
	InvalidResponse(serde_json::Error),
	/// mayland returned an error
	MaylandError(mayland_comm::Error),
	/// error parsing config file
	Mayfig(mayland_config::error::MayfigError),
	/// config was not found
	ConfigNotFound(PathBuf),
	/// mayctl wasn't started inside mayland
	MaylandNotRunning,
	UnexpectedResponse {
		expected: &'static str,
		actual: &'static str,
	},
}

impl Termination for Term {
	fn report(self) -> std::process::ExitCode {
		if matches!(self, Term::Ok) {
			std::process::ExitCode::SUCCESS
		} else {
			let mut stderr = anstream::stderr();
			let _ = write!(stderr, "{self}");

			std::process::ExitCode::FAILURE
		}
	}
}

impl Display for Term {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Term::Ok => Ok(()),
			Term::IoError(path, err) => {
				writeln!(
					f,
					"{}: {}",
					"error".bright_red().bold(),
					format_args!("io error reading {}", path.display()).bold()
				)?;
				writeln!(f, "  {} {}", "::".bright_blue().bold(), err)
			}
			Term::NotFound(socket_path) => {
				writeln!(
					f,
					"{}: {}",
					"error".bright_red().bold(),
					"socket not found".bold()
				)?;
				writeln!(
					f,
					"  {} file {} does not exist",
					"::".bright_blue().bold(),
					socket_path.display()
				)
			}
			Term::InvalidResponse(err) => {
				writeln!(
					f,
					"{}: {}",
					"error".bright_red().bold(),
					"couldn't deserialize mayland response".bold()
				)?;
				writeln!(f, "  {} {}", "::".bright_blue().bold(), err)?;
				writeln!(
					f,
					"  {} is your version of mayctl up-to-date?",
					"::".bright_blue().bold()
				)?;
				writeln!(
					f,
					"  {} did you restart mayland after updating?",
					"::".bright_blue().bold()
				)
			}
			Term::MaylandError(err) => {
				writeln!(
					f,
					"{}: {}",
					"error".bright_red().bold(),
					"mayland returned an error".bold()
				)?;
				writeln!(f, "  {} {}", "::".bright_blue().bold(), err)
			}
			Term::Mayfig(mayfig) => {
				writeln!(f, "{mayfig}")?;
				writeln!(
					f,
					"{}: {}",
					"error".bright_red().bold(),
					"failed to deserialize config".bold()
				)
			}
			Term::ConfigNotFound(path) => {
				writeln!(
					f,
					"{}: {}",
					"error".bright_red().bold(),
					format_args!("config {} not found", path.display()).bold()
				)
			}
			Term::MaylandNotRunning => {
				writeln!(
					f,
					"{}: {}",
					"error".bright_red().bold(),
					"mayland not running".bold()
				)?;
				writeln!(
					f,
					"  {} env ${} not set",
					"::".bright_blue().bold(),
					MAYLAND_SOCKET_VAR
				)
			}
			Term::UnexpectedResponse { expected, actual } => {
				writeln!(
					f,
					"{}: {}",
					"error".bright_red().bold(),
					"mayland returned unexpected response".bold()
				)?;
				writeln!(
					f,
					"  {} expected {}, got {}",
					"::".bright_blue().bold(),
					expected,
					actual
				)
			}
		}
	}
}

impl From<mayland_comm::Error> for Term {
	fn from(value: mayland_comm::Error) -> Self {
		if let mayland_comm::Error::FailedToReadConfig(path) = &value {
			if let Err(err) = mayland_config::Config::read(path, mayland_config::bind::CompMod::Meta) {
				return Term::from(err);
			}
		}

		Term::MaylandError(value)
	}
}

impl From<mayland_config::Error> for Term {
	fn from(value: mayland_config::Error) -> Self {
		match value {
			mayland_config::Error::IoError(path, err) => Term::IoError(path, err),
			mayland_config::Error::Mayfig(mayfig) => Term::Mayfig(mayfig),
			mayland_config::Error::NotFound(path) => Term::ConfigNotFound(path),
		}
	}
}

macro_rules! ensure_matches {
	($left:expr, $( $pattern:pat_param )|+, $expected:literal) => {
		match $left {
			$( $pattern )|+ => {}
			ref left => {
				let actual = $crate::term::get_response_name(left);
				return Term::UnexpectedResponse {
					expected: $expected,
					actual,
				}
			}
		}
	}
}

pub(crate) use ensure_matches;

macro_rules! unexpected {
	($left:expr, $expected:literal) => {{
		let actual = $crate::term::get_response_name(&$left);
		return Term::UnexpectedResponse {
			expected: $expected,
			actual,
		};
	}};
}

pub(crate) use unexpected;

/// get a static str, that describes the response, for use
/// in [`Term::UnexpectedResponse`].
#[doc(hidden)]
pub(crate) fn get_response_name(response: &Response) -> &'static str {
	match response {
		Response::Err(_) => unreachable!("the error case should have been handled first"),
		Response::Dispatch => "dispatch",
		Response::Reload => "reload",
		Response::Devices(_) => "devices",
		Response::Outputs(_) => "outputs",
		Response::Workspaces(_) => "workspaces",
	}
}
