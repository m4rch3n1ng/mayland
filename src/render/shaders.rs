use smithay::backend::renderer::gles::{GlesPixelProgram, GlesRenderer, UniformName, UniformType};

pub static OUTLINE_SHADER: &str = include_str!("./shaders/focusring.frag");

#[derive(Debug, Clone)]
pub struct Shaders {
	pub border: GlesPixelProgram,
}

pub fn init(renderer: &mut GlesRenderer) {
	let shaders = Shaders::compile(renderer);

	let user_data = renderer.egl_context().user_data();
	user_data.insert_if_missing(|| shaders);
}

impl Shaders {
	fn compile(renderer: &mut GlesRenderer) -> Self {
		let border = renderer
			.compile_custom_pixel_shader(
				OUTLINE_SHADER,
				&[
					UniformName::new("color", UniformType::_3f),
					UniformName::new("thickness", UniformType::_1f),
				],
			)
			.unwrap();

		Shaders { border }
	}

	pub fn get(renderer: &GlesRenderer) -> Self {
		let user_data = renderer.egl_context().user_data();
		user_data.get::<Shaders>().cloned().unwrap()
	}
}
