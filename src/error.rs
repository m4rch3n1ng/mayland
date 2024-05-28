use thiserror::Error;

#[derive(Debug, Error)]
pub enum MaylandError {
	#[error("couldn't find file {0:?}")]
	FileNotFound(String),

	#[error("")]
	AlreadyPrinted,
}
