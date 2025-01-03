use crate::CONFIG_PATH;
use annotate_snippets::{Level, Renderer, Snippet};
use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
	IoError(std::io::Error),
	NotFound,
	Mayfig { error: mayfig::Error, file: String },
}

impl Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Error::IoError(io_err) => writeln!(f, "error reading config: {}", io_err),
			Error::NotFound => f.write_str("config not found"),
			Error::Mayfig { error, file } => {
				let code = error.code().to_string();
				let path = &*CONFIG_PATH.to_string_lossy();

				let message = if let Some(span) = error.span() {
					Level::Error.title(&code).snippet(
						Snippet::source(file)
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
	}
}

impl std::error::Error for Error {}
