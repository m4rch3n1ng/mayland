use super::shaders::Shaders;
use smithay::{
	backend::renderer::{
		element::Kind,
		gles::{Uniform, element::PixelShaderElement},
		glow::GlowRenderer,
	},
	utils::{Logical, Point, Rectangle, Size},
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
		let t = i32::from(thickness);
		area.loc -= Point::from((t, t));
		area.size += Size::from((t * 2, t * 2));

		let shaders = Shaders::get(renderer.borrow_mut());
		PixelShaderElement::new(
			shaders.border,
			area,
			None,
			1.0,
			vec![
				Uniform::new("color", color),
				Uniform::new("thickness", f32::from(thickness)),
			],
			Kind::Unspecified,
		)
	}
}
