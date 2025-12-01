use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Layout {
	pub tiling: Tiling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Tiling {
	pub gaps: u8,
	pub border: u8,
}

impl Default for Tiling {
	fn default() -> Self {
		Tiling { gaps: 10, border: 20 }
	}
}
