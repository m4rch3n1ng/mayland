use super::shaders::Shaders;
use smithay::{
	backend::renderer::{
		element::Kind,
		gles::{element::PixelShaderElement, Uniform},
		glow::GlowRenderer,
	},
	utils::{Logical, Rectangle},
};
use std::borrow::BorrowMut;

pub struct FocusRing;

impl FocusRing {
	pub fn element(
		renderer: &mut GlowRenderer,
		mut area: Rectangle<i32, Logical>,
		color: [f32; 3],
		thickness: u8,
	) -> PixelShaderElement {
		let thickness = i32::from(thickness);
		area.loc -= (thickness, thickness).into();
		area.size += (thickness * 2, thickness * 2).into();

		let shaders = Shaders::get(renderer.borrow_mut());
		PixelShaderElement::new(
			shaders.border,
			area,
			None,
			1.0,
			vec![
				Uniform::new("color", color),
				Uniform::new("thickness", thickness as f32),
			],
			Kind::Unspecified,
		)
	}
}
