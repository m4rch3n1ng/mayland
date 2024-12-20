use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("io error")]
	IoError(#[from] std::io::Error),
	#[error("file not found")]
	NotFound,
	#[error("error parsing mayfig file")]
	Mayfig { error: mayfig::Error, file: String },
	#[error("")]
	AlreadyPrinted,
}
