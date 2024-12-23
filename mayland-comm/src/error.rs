use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// a mayland error
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Error {
	/// the request was invalid
	InvalidRequest,
	/// the config couldn't be read
	FailedToReadConfig,
}

impl std::error::Error for Error {}

impl Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Error::InvalidRequest => write!(f, "invalid request"),
			Error::FailedToReadConfig => write!(f, "failed to read config"),
		}
	}
}
