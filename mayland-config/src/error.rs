use annotate_snippets::{AnnotationKind, Group, Level, Renderer, Snippet};
use owo_colors::OwoColorize;
use std::{fmt::Display, path::PathBuf};

#[derive(Debug)]
pub struct MayfigError {
	pub error: Box<mayfig::Error>,
	pub path: PathBuf,
	pub file: String,
}

#[derive(Debug)]
pub enum Error {
	IoError(PathBuf, std::io::Error),
	NotFound(PathBuf),
	Mayfig(MayfigError),
}

impl Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Error::IoError(path, err) => write!(
				f,
				"{}: {} ({err})",
				"error".bright_red().bold(),
				format_args!("failed to read config {}", path.display()).bold()
			),
			Error::NotFound(path) => write!(
				f,
				"{}: {}",
				"error".bright_red().bold(),
				format_args!("config {} not found", path.display()).bold(),
			),
			Error::Mayfig(mayfig) => write!(f, "{mayfig}"),
		}
	}
}

impl std::error::Error for Error {}

impl Display for MayfigError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let code = self.error.code().to_string();
		let path = self.path.to_string_lossy();

		let message = if let Some(span) = self.error.span() {
			Level::ERROR.primary_title(&code).element(
				Snippet::source(&self.file)
					.path(&path)
					.fold(true)
					.annotation(AnnotationKind::Primary.span(span.range())),
			)
		} else {
			Group::with_title(Level::ERROR.primary_title(&code))
		};

		let renderer = Renderer::styled();
		write!(f, "{}", renderer.render(&[message]))?;

		Ok(())
	}
}

impl From<MayfigError> for Error {
	fn from(value: MayfigError) -> Self {
		Error::Mayfig(value)
	}
}

impl std::error::Error for MayfigError {}
