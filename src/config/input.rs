use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Input {
	pub xkb: Xkb,
}

#[derive(Debug, Default, Deserialize)]
pub struct Xkb {
	pub file: Option<String>,
}
