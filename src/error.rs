use mayland_config::Error as ConfigError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MaylandError {
	#[error("config error")]
	ConfigError(#[source] ConfigError),

	#[error("")]
	AlreadyPrinted,
}

impl From<ConfigError> for MaylandError {
	fn from(value: ConfigError) -> Self {
		match value {
			ConfigError::AlreadyPrinted => MaylandError::AlreadyPrinted,
			config_error => MaylandError::ConfigError(config_error),
		}
	}
}
