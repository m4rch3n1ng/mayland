use smithay::{
	backend::renderer::{
		element::Kind,
		gles::{element::PixelShaderElement, GlesRenderer, Uniform, UniformName, UniformType},
		glow::GlowRenderer,
	},
	utils::{Logical, Rectangle},
};
use std::borrow::BorrowMut;

pub static OUTLINE_SHADER: &str = include_str!("./shaders/focusring.frag");

pub struct FocusRing;

impl FocusRing {
	pub fn unfocussed(renderer: &mut GlowRenderer, mut area: Rectangle<i32, Logical>) -> PixelShaderElement {
		let thickness = 4;
		area.loc -= (thickness, thickness).into();
		area.size += (thickness * 2, thickness * 2).into();

		let gles: &mut GlesRenderer = renderer.borrow_mut();
		let shader = gles
			.compile_custom_pixel_shader(
				OUTLINE_SHADER,
				&[
					UniformName::new("color", UniformType::_3f),
					UniformName::new("thickness", UniformType::_1f),
				],
			)
			.unwrap();

		PixelShaderElement::new(
			shader,
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
