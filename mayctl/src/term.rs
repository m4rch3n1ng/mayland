use mayland_comm::{Response, MAYLAND_SOCKET_VAR};
use owo_colors::OwoColorize;
use std::{fmt::Display, io::Write as _, process::Termination};

pub enum Term {
	/// mayctl exited successfully
	Ok,
	/// an io error occured
	IoError(std::io::Error),
	/// the mayland socket was not found
	NotFound(String),
	/// mayland returned a reply that couldn't be deserialized
	InvalidReply(serde_json::Error),
	/// mayland returned an error
	MaylandError(mayland_comm::Error),
	/// mayctl wasn't started inside mayland
	MaylandNotRunning,
	UnexpectedResponse {
		expected: &'static str,
		actual: &'static str,
	},
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
			Term::IoError(err) => {
				writeln!(f, "{}: {}", "error".red().bold(), "io error".bold())?;
				writeln!(f, "  {} {}", "::".blue().bold(), err)
			}
			Term::NotFound(socket_path) => {
				writeln!(f, "{}: {}", "error".red().bold(), "socket not found".bold())?;
				writeln!(f, "  {} file {} does not exist", "::".blue().bold(), socket_path)
			}
			Term::InvalidReply(err) => {
				writeln!(
					f,
					"{}: {}",
					"error".red().bold(),
					"couldn't deserialize mayland reply".bold()
				)?;
				writeln!(f, "  {} {}", "::".blue().bold(), err)?;
				writeln!(
					f,
					"  {} is your version of mayctl up-to-date?",
					"::".blue().bold()
				)?;
				writeln!(
					f,
					"  {} did you restart mayland after updating?",
					"::".blue().bold()
				)
			}
			Term::MaylandError(err) => {
				writeln!(
					f,
					"{}: {}",
					"error".red().bold(),
					"mayland returned an error".bold()
				)?;
				writeln!(f, "  {} {}", "::".blue().bold(), err)
			}
			Term::MaylandNotRunning => {
				writeln!(f, "{}: {}", "error".red().bold(), "mayland not running".bold())?;
				writeln!(f, "  {} env ${} not set", "::".blue().bold(), MAYLAND_SOCKET_VAR)
			}
			Term::UnexpectedResponse { expected, actual } => {
				writeln!(
					f,
					"{}: {}",
					"error".red().bold(),
					"mayland returned unexpected response".bold()
				)?;
				writeln!(
					f,
					"  {} expected {}, got {}",
					"::".blue().bold(),
					expected,
					actual
				)
			}
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
		Response::Workspaces(_) => "workspaces",
	}
}
