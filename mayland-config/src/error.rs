use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("couldn't find file {0:?}")]
	FileNotFound(String),

	#[error("")]
	AlreadyPrinted,
}
