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
	pub fn unfocussed(renderer: &mut GlowRenderer, mut area: Rectangle<i32, Logical>) -> PixelShaderElement {
		let thickness = 4;
		area.loc -= (thickness, thickness).into();
		area.size += (thickness * 2, thickness * 2).into();

		let shaders = Shaders::get(renderer.borrow_mut());
		PixelShaderElement::new(
			shaders.border,
			area,
			None,
			1.0,
			vec![
				Uniform::new("color", [1.0, 0.0, 1.0]),
				Uniform::new("thickness", thickness as f32),
			],
			Kind::Unspecified,
		)
	}
}
