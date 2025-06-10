use super::shaders::Shaders;
use mayland_config::decoration::Color;
use smithay::{
	backend::renderer::{
		element::Kind,
		gles::{Uniform, element::PixelShaderElement},
		glow::GlowRenderer,
	},
	utils::{Logical, Point, Rectangle, Size},
};
use std::borrow::Borrow;

pub struct FocusRing;

impl FocusRing {
	pub fn element(
		renderer: &GlowRenderer,
		mut area: Rectangle<i32, Logical>,
		color: Color,
		thickness: u8,
	) -> PixelShaderElement {
		let t = i32::from(thickness);
		area.loc -= Point::new(t, t);
		area.size += Size::new(t * 2, t * 2);

		let shaders = Shaders::get(renderer.borrow());
		PixelShaderElement::new(
			shaders.border,
			area,
			None,
			1.0,
			vec![
				Uniform::new("color", color.as_f32s()),
				Uniform::new("thickness", f32::from(thickness)),
			],
			Kind::Unspecified,
		)
	}
}
