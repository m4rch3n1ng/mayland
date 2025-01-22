use crate::CONFIG_PATH;
use annotate_snippets::{Level, Renderer, Snippet};
use std::fmt::Display;

#[derive(Debug)]
pub struct MayfigError {
	pub error: mayfig::Error,
	pub file: String,
}

#[derive(Debug)]
pub enum Error {
	IoError(std::io::Error),
	NotFound,
	Mayfig(MayfigError),
}

impl Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Error::IoError(io_err) => write!(f, "error reading config: {}", io_err),
			Error::NotFound => f.write_str("config not found"),
			Error::Mayfig(mayfig) => write!(f, "{}", mayfig),
		}
	}
}

impl std::error::Error for Error {}

impl Display for MayfigError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let code = self.error.code().to_string();
		let path = &*CONFIG_PATH.to_string_lossy();

		let message = if let Some(span) = self.error.span() {
			Level::Error.title(&code).snippet(
				Snippet::source(&self.file)
					.origin(path)
					.fold(true)
					.annotation(Level::Error.span(span.range())),
			)
		} else {
			Level::Error.title(&code)
		};

		let renderer = Renderer::styled();
		write!(f, "{}", renderer.render(message))?;

		Ok(())
	}
}

impl From<MayfigError> for Error {
	fn from(value: MayfigError) -> Self {
		Error::Mayfig(value)
	}
}

impl std::error::Error for MayfigError {}
