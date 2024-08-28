use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("io error")]
	IoError(#[from] std::io::Error),
	#[error("")]
	AlreadyPrinted,
}
