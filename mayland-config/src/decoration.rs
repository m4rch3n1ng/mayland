use serde::{de::Visitor, Deserialize};

#[derive(Debug, Clone, Copy, Default, Deserialize)]
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

#[derive(Debug, Clone, Copy)]
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

impl<'de> Deserialize<'de> for Color {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		deserializer.deserialize_str(ColorVis)
	}
}

struct ColorVis;

impl Visitor<'_> for ColorVis {
	type Value = Color;

	fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str("a mayfig color")
	}

	fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
		let hex = hex_color(v)
			.ok_or_else(|| serde::de::Error::custom(format_args!("invalid hex color {:?}", v)))?;
		Ok(Color(hex))
	}
}

fn hex_digit(c: u8) -> Option<u8> {
	match c {
		b'0'..=b'9' => Some(c - b'0'),
		b'A'..=b'F' => Some(c - b'A' + 10),
		b'a'..=b'f' => Some(c - b'a' + 10),
		_ => None,
	}
}

fn hex_color(s: &str) -> Option<[u8; 3]> {
	let hex = s.strip_prefix("#")?;
	if let [r1, r2, g1, g2, b1, b2] = hex.as_bytes() {
		let color = [
			hex_digit(*r1)? * 16 + hex_digit(*r2)?,
			hex_digit(*g1)? * 16 + hex_digit(*g2)?,
			hex_digit(*b1)? * 16 + hex_digit(*b2)?,
		];
		Some(color)
	} else {
		None
	}
}