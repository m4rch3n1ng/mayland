use smithay::{
	backend::renderer::{
		element::{
			memory::MemoryRenderBufferRenderElement, surface::WaylandSurfaceRenderElement,
			utils::CropRenderElement, Element, Id, RenderElement, UnderlyingStorage,
		},
		gles::element::PixelShaderElement,
		glow::GlowRenderer,
		utils::{CommitCounter, DamageSet, OpaqueRegions},
		ImportAll, ImportMem, Renderer,
	},
	utils::{Physical, Point, Rectangle, Scale, Transform},
};
use std::fmt::Debug;

mod focusring;

pub use focusring::FocusRing;

pub type MaylandRenderElements = OutputRenderElements<GlowRenderer>;

pub enum OutputRenderElements<R: Renderer> {
	DefaultPointer(MemoryRenderBufferRenderElement<R>),
	CropSurface(CropRenderElement<WaylandSurfaceRenderElement<R>>),
	Surface(WaylandSurfaceRenderElement<R>),
	FocusElement(PixelShaderElement),
}

impl<R> Element for OutputRenderElements<R>
where
	R: Renderer,
	<R as Renderer>::TextureId: 'static,
	R: ImportAll + ImportMem,
{
	fn id(&self) -> &Id {
		match self {
			Self::DefaultPointer(x) => x.id(),
			Self::CropSurface(x) => x.id(),
			Self::Surface(x) => x.id(),
			Self::FocusElement(x) => x.id(),
		}
	}

	fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
		match self {
			Self::DefaultPointer(x) => x.location(scale),
			Self::CropSurface(x) => x.location(scale),
			Self::Surface(x) => x.location(scale),
			Self::FocusElement(x) => x.location(scale),
		}
	}

	fn src(&self) -> Rectangle<f64, smithay::utils::Buffer> {
		match self {
			Self::DefaultPointer(x) => x.src(),
			Self::CropSurface(x) => x.src(),
			Self::Surface(x) => x.src(),
			Self::FocusElement(x) => x.src(),
		}
	}

	fn transform(&self) -> Transform {
		match self {
			Self::DefaultPointer(x) => x.transform(),
			Self::CropSurface(x) => x.transform(),
			Self::Surface(x) => x.transform(),
			Self::FocusElement(x) => x.transform(),
		}
	}

	fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
		match self {
			Self::DefaultPointer(x) => x.geometry(scale),
			Self::CropSurface(x) => x.geometry(scale),
			Self::Surface(x) => x.geometry(scale),
			Self::FocusElement(x) => x.geometry(scale),
		}
	}

	fn current_commit(&self) -> CommitCounter {
		match self {
			Self::DefaultPointer(x) => x.current_commit(),
			Self::CropSurface(x) => x.current_commit(),
			Self::Surface(x) => x.current_commit(),
			Self::FocusElement(x) => x.current_commit(),
		}
	}

	fn damage_since(&self, scale: Scale<f64>, commit: Option<CommitCounter>) -> DamageSet<i32, Physical> {
		match self {
			Self::DefaultPointer(x) => x.damage_since(scale, commit),
			Self::CropSurface(x) => x.damage_since(scale, commit),
			Self::Surface(x) => x.damage_since(scale, commit),
			Self::FocusElement(x) => x.damage_since(scale, commit),
		}
	}

	fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
		match self {
			Self::DefaultPointer(x) => x.opaque_regions(scale),
			Self::CropSurface(x) => x.opaque_regions(scale),
			Self::Surface(x) => x.opaque_regions(scale),
			Self::FocusElement(x) => x.opaque_regions(scale),
		}
	}

	fn alpha(&self) -> f32 {
		match self {
			Self::DefaultPointer(x) => x.alpha(),
			Self::CropSurface(x) => x.alpha(),
			Self::Surface(x) => x.alpha(),
			Self::FocusElement(x) => x.alpha(),
		}
	}

	fn kind(&self) -> smithay::backend::renderer::element::Kind {
		match self {
			Self::DefaultPointer(x) => x.kind(),
			Self::CropSurface(x) => x.kind(),
			Self::Surface(x) => x.kind(),
			Self::FocusElement(x) => x.kind(),
		}
	}
}

impl RenderElement<GlowRenderer> for OutputRenderElements<GlowRenderer> {
	fn draw(
		&self,
		frame: &mut <GlowRenderer as Renderer>::Frame<'_>,
		src: Rectangle<f64, smithay::utils::Buffer>,
		dst: Rectangle<i32, Physical>,
		damage: &[Rectangle<i32, Physical>],
		opaque_regions: &[Rectangle<i32, Physical>],
	) -> Result<(), <GlowRenderer as Renderer>::Error> {
		match self {
			Self::DefaultPointer(x) => x.draw(frame, src, dst, damage, opaque_regions),
			Self::CropSurface(x) => x.draw(frame, src, dst, damage, opaque_regions),
			Self::Surface(x) => x.draw(frame, src, dst, damage, opaque_regions),
			Self::FocusElement(x) => {
				RenderElement::<GlowRenderer>::draw(x, frame, src, dst, damage, opaque_regions)
			}
		}
	}

	#[inline]
	fn underlying_storage(&self, renderer: &mut GlowRenderer) -> Option<UnderlyingStorage<'_>> {
		match self {
			Self::DefaultPointer(x) => x.underlying_storage(renderer),
			Self::CropSurface(x) => x.underlying_storage(renderer),
			Self::Surface(x) => x.underlying_storage(renderer),
			Self::FocusElement(x) => x.underlying_storage(renderer),
		}
	}
}

impl<R: Renderer> From<MemoryRenderBufferRenderElement<R>> for OutputRenderElements<R> {
	#[inline]
	fn from(field: MemoryRenderBufferRenderElement<R>) -> OutputRenderElements<R> {
		OutputRenderElements::DefaultPointer(field)
	}
}

impl<R: Renderer> From<CropRenderElement<WaylandSurfaceRenderElement<R>>> for OutputRenderElements<R> {
	#[inline]
	fn from(field: CropRenderElement<WaylandSurfaceRenderElement<R>>) -> OutputRenderElements<R> {
		OutputRenderElements::CropSurface(field)
	}
}

impl<R: Renderer> From<WaylandSurfaceRenderElement<R>> for OutputRenderElements<R> {
	#[inline]
	fn from(field: WaylandSurfaceRenderElement<R>) -> OutputRenderElements<R> {
		OutputRenderElements::Surface(field)
	}
}

impl<R: Renderer> From<PixelShaderElement> for OutputRenderElements<R> {
	#[inline]
	fn from(field: PixelShaderElement) -> OutputRenderElements<R> {
		OutputRenderElements::FocusElement(field)
	}
}

impl<R: Renderer> Debug for OutputRenderElements<R> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			OutputRenderElements::DefaultPointer(element) => {
				f.debug_tuple("DefaultPointer").field(&element).finish()
			}
			OutputRenderElements::CropSurface(surface) => {
				f.debug_tuple("CropSurface").field(&surface).finish()
			}
			OutputRenderElements::Surface(surface) => f.debug_tuple("Surface").field(&surface).finish(),
			OutputRenderElements::FocusElement(element) => {
				f.debug_tuple("FocusElement").field(&element).finish()
			}
		}
	}
}
