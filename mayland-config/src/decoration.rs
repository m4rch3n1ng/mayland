use serde::{de::Visitor, Deserialize};
use smithay::backend::renderer::Color32F;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Decoration {
	pub background: Color,
	pub focus: Focus,
}

impl Default for Decoration {
	fn default() -> Self {
		Decoration {
			background: Color::BACKGROUND,
			focus: Focus::default(),
		}
	}
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
	/// default active border color
	///
	/// tailwind "fuchsia-700"
	const ACTIVE: Color = Color([0xa2, 0x1c, 0xaf]);
	/// default inactive border color
	///
	/// tailwind "zinc-500"
	const INACTIVE: Color = Color([0x71, 0x71, 0x7a]);
	/// default background color
	///
	/// css "teal"
	const BACKGROUND: Color = Color([0x00, 0x80, 0x80]);
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
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		deserializer.deserialize_str(ColorVis)
	}
}

impl From<Color> for Color32F {
	fn from(color: Color) -> Self {
		Color32F::new(
			color.0[0] as f32 / 255.0,
			color.0[1] as f32 / 255.0,
			color.0[2] as f32 / 255.0,
			1.0,
		)
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
