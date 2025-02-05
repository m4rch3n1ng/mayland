use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Layout {
	pub tiling: Tiling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Tiling {
	pub border: u8,
	pub gaps: u8,
}

impl Default for Tiling {
	fn default() -> Self {
		Tiling { border: 20, gaps: 10 }
	}
}
