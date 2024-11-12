use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Decoration {
	pub focus: Focus,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Focus {
	pub active: Color,
	pub inactive: Color,
	pub thickness: u8,
}

impl Default for Focus {
	fn default() -> Self {
		Focus {
			active: Color::ACTIVE,
			inactive: Color::INACTIVE,
			thickness: 4,
		}
	}
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Color([u8; 3]);

impl Color {
	const ACTIVE: Color = Color([0xa2, 0x1c, 0xaf]);
	const INACTIVE: Color = Color([0x71, 0x71, 0x7a]);
}

impl Color {
	pub const fn as_f32s(self) -> [f32; 3] {
		[
			self.0[0] as f32 / 255.0,
			self.0[1] as f32 / 255.0,
			self.0[2] as f32 / 255.0,
		]
	}
}
