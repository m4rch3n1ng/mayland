use smithay::{
	backend::renderer::{
		element::{
			memory::MemoryRenderBufferRenderElement, surface::WaylandSurfaceRenderElement,
			utils::CropRenderElement,
		},
		glow::GlowRenderer,
		ImportAll, ImportMem,
	},
	render_elements,
};
use std::fmt::Debug;

pub type MaylandRenderElements = OutputRenderElements<GlowRenderer>;

render_elements! {
	pub OutputRenderElements<R> where R: ImportAll + ImportMem;
	DefaultPointer = MemoryRenderBufferRenderElement<R>,
	CropSurface = CropRenderElement<WaylandSurfaceRenderElement<R>>,
	Surface = WaylandSurfaceRenderElement<R>,
}

impl<R: ImportAll + ImportMem> Debug for OutputRenderElements<R> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			OutputRenderElements::DefaultPointer(element) => {
				f.debug_tuple("DefaultPointer").field(&element).finish()
			}
			OutputRenderElements::CropSurface(surface) => {
				f.debug_tuple("CropSurface").field(&surface).finish()
			}
			OutputRenderElements::Surface(surface) => f.debug_tuple("Surface").field(&surface).finish(),
			OutputRenderElements::_GenericCatcher(_) => f.write_str("_GenericCatcher"),
		}
	}
}
