use smithay::{
	backend::{
		allocator::Fourcc,
		renderer::{
			element::{
				memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
				surface::WaylandSurfaceRenderElement,
				RenderElement,
			},
			glow::GlowRenderer,
			ImportAll, ImportMem,
		},
	},
	desktop::space::SpaceRenderElements,
	render_elements,
	utils::{Physical, Point, Transform},
};
use std::fmt::Debug;
use xcursor::parser::Image;

const FALLBACK_CURSOR_DATA: &[u8] = include_bytes!("../resources/cursor.rgba");

fn load_default_cursor() -> (MemoryRenderBuffer, Point<i32, Physical>) {
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

	let mem = MemoryRenderBuffer::from_slice(
		&icon.pixels_rgba,
		Fourcc::Argb8888,
		(icon.width as i32, icon.height as i32),
		2,
		Transform::Normal,
		None,
	);

	let hotspot = Point::from((icon.xhot as i32, icon.yhot as i32));

	(mem, hotspot)
}

pub struct CursorBuffer(Option<(MemoryRenderBuffer, Point<i32, Physical>)>);

impl Debug for CursorBuffer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("Cursor").field(&..).finish()
	}
}

impl CursorBuffer {
	pub const fn new() -> Self {
		CursorBuffer(None)
	}

	fn get(&mut self) -> &(MemoryRenderBuffer, Point<i32, Physical>) {
		self.0.get_or_insert_with(load_default_cursor)
	}

	pub fn buffer(&mut self) -> MemoryRenderBuffer {
		self.get().0.clone()
	}
}

pub type MaylandRenderElements =
	OutputRenderElements<GlowRenderer, WaylandSurfaceRenderElement<GlowRenderer>>;

render_elements! {
	pub OutputRenderElements<R, E> where
		R: ImportAll + ImportMem;
	DefaultPointer = MemoryRenderBufferRenderElement<R>,
	Space=SpaceRenderElements<R, E>,
}

impl<R: ImportAll + ImportMem, E: RenderElement<R>> Debug for OutputRenderElements<R, E> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			OutputRenderElements::Space(_) => f.debug_tuple("Space").field(&..).finish(),
			OutputRenderElements::DefaultPointer(element) => {
				f.debug_tuple("DefaultPointer").field(&element).finish()
			}
			OutputRenderElements::_GenericCatcher(_) => f.write_str("_GenericCatcher"),
		}
	}
}
