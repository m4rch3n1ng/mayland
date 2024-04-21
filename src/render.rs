use smithay::{
	backend::{
		allocator::Fourcc,
		renderer::{
			element::{
				memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
				Kind,
			},
			glow::GlowRenderer,
			ImportAll, ImportMem,
		},
	},
	desktop::space::SpaceRenderElements,
	render_elements,
	utils::{Physical, Point, Transform},
};
use xcursor::parser::Image;

const FALLBACK_CURSOR_DATA: &[u8] = include_bytes!("../resources/cursor.rgba");

pub struct Cursor(Image);

impl Cursor {
	pub fn load() -> Self {
		let icon = Image {
			size: 32,
			width: 64,
			height: 64,
			xhot: 1,
			yhot: 1,
			delay: 0,
			pixels_rgba: Vec::from(FALLBACK_CURSOR_DATA),
			pixels_argb: vec![],
		};

		Cursor(icon)
	}

	fn buffer(&self) -> MemoryRenderBuffer {
		MemoryRenderBuffer::from_slice(
			&self.0.pixels_rgba,
			Fourcc::Argb8888,
			(self.0.width as i32, self.0.height as i32),
			2,
			Transform::Normal,
			None,
		)
	}

	pub fn element(
		&self,
		renderer: &mut GlowRenderer,
		position: Point<i32, Physical>,
	) -> MemoryRenderBufferRenderElement<GlowRenderer> {
		let texture = self.buffer();

		MemoryRenderBufferRenderElement::from_buffer(
			renderer,
			position.to_f64(),
			&texture,
			None,
			None,
			None,
			Kind::Cursor,
		)
		.unwrap()
	}
}

render_elements! {
	pub MaylandRenderElements<R, E> where
		R: ImportAll + ImportMem;
	DefaultPointer = MemoryRenderBufferRenderElement<R>,
	Space=SpaceRenderElements<R, E>,
}
