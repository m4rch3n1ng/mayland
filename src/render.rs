use smithay::backend::renderer::{
	element::{
		memory::MemoryRenderBufferRenderElement, surface::WaylandSurfaceRenderElement,
		utils::CropRenderElement, RenderElement,
	},
	gles::element::PixelShaderElement,
	glow::GlowRenderer,
	ImportAll, ImportMem,
};
use std::fmt::Debug;

mod focusring;

pub use focusring::FocusRing;

pub type MaylandRenderElements = OutputRenderElements<GlowRenderer>;

pub enum OutputRenderElements<R>
where
	R: smithay::backend::renderer::Renderer,
{
	DefaultPointer(MemoryRenderBufferRenderElement<R>),
	CropSurface(CropRenderElement<WaylandSurfaceRenderElement<R>>),
	Surface(WaylandSurfaceRenderElement<R>),
	FocusElement(PixelShaderElement),
	#[doc(hidden)]
	_GenericCatcher((std::marker::PhantomData<R>, std::convert::Infallible)),
}
impl<R> smithay::backend::renderer::element::Element for OutputRenderElements<R>
where
	R: smithay::backend::renderer::Renderer,
	<R as smithay::backend::renderer::Renderer>::TextureId: 'static,
	R: ImportAll + ImportMem,
{
	fn id(&self) -> &smithay::backend::renderer::element::Id {
		match self {
			Self::DefaultPointer(x) => smithay::backend::renderer::element::Element::id(x),

			Self::CropSurface(x) => smithay::backend::renderer::element::Element::id(x),

			Self::Surface(x) => smithay::backend::renderer::element::Element::id(x),

			Self::FocusElement(x) => smithay::backend::renderer::element::Element::id(x),
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}

	fn location(
		&self,
		scale: smithay::utils::Scale<f64>,
	) -> smithay::utils::Point<i32, smithay::utils::Physical> {
		match self {
			Self::DefaultPointer(x) => smithay::backend::renderer::element::Element::location(x, scale),
			Self::CropSurface(x) => smithay::backend::renderer::element::Element::location(x, scale),
			Self::Surface(x) => smithay::backend::renderer::element::Element::location(x, scale),
			Self::FocusElement(x) => smithay::backend::renderer::element::Element::location(x, scale),
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}

	fn src(&self) -> smithay::utils::Rectangle<f64, smithay::utils::Buffer> {
		match self {
			Self::DefaultPointer(x) => smithay::backend::renderer::element::Element::src(x),

			Self::CropSurface(x) => smithay::backend::renderer::element::Element::src(x),

			Self::Surface(x) => smithay::backend::renderer::element::Element::src(x),

			Self::FocusElement(x) => smithay::backend::renderer::element::Element::src(x),
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}

	fn transform(&self) -> smithay::utils::Transform {
		match self {
			Self::DefaultPointer(x) => smithay::backend::renderer::element::Element::transform(x),
			Self::CropSurface(x) => smithay::backend::renderer::element::Element::transform(x),
			Self::Surface(x) => smithay::backend::renderer::element::Element::transform(x),
			Self::FocusElement(x) => smithay::backend::renderer::element::Element::transform(x),
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}

	fn geometry(
		&self,
		scale: smithay::utils::Scale<f64>,
	) -> smithay::utils::Rectangle<i32, smithay::utils::Physical> {
		match self {
			Self::DefaultPointer(x) => smithay::backend::renderer::element::Element::geometry(x, scale),
			Self::CropSurface(x) => smithay::backend::renderer::element::Element::geometry(x, scale),
			Self::Surface(x) => smithay::backend::renderer::element::Element::geometry(x, scale),
			Self::FocusElement(x) => smithay::backend::renderer::element::Element::geometry(x, scale),
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}

	fn current_commit(&self) -> smithay::backend::renderer::utils::CommitCounter {
		match self {
			Self::DefaultPointer(x) => smithay::backend::renderer::element::Element::current_commit(x),
			Self::CropSurface(x) => smithay::backend::renderer::element::Element::current_commit(x),
			Self::Surface(x) => smithay::backend::renderer::element::Element::current_commit(x),
			Self::FocusElement(x) => smithay::backend::renderer::element::Element::current_commit(x),
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}

	fn damage_since(
		&self,
		scale: smithay::utils::Scale<f64>,
		commit: Option<smithay::backend::renderer::utils::CommitCounter>,
	) -> smithay::backend::renderer::utils::DamageSet<i32, smithay::utils::Physical> {
		match self {
			Self::DefaultPointer(x) => {
				smithay::backend::renderer::element::Element::damage_since(x, scale, commit)
			}

			Self::CropSurface(x) => {
				smithay::backend::renderer::element::Element::damage_since(x, scale, commit)
			}

			Self::Surface(x) => smithay::backend::renderer::element::Element::damage_since(x, scale, commit),

			Self::FocusElement(x) => {
				smithay::backend::renderer::element::Element::damage_since(x, scale, commit)
			}
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}

	fn opaque_regions(
		&self,
		scale: smithay::utils::Scale<f64>,
	) -> smithay::backend::renderer::utils::OpaqueRegions<i32, smithay::utils::Physical> {
		match self {
			Self::DefaultPointer(x) => smithay::backend::renderer::element::Element::opaque_regions(x, scale),

			Self::CropSurface(x) => smithay::backend::renderer::element::Element::opaque_regions(x, scale),

			Self::Surface(x) => smithay::backend::renderer::element::Element::opaque_regions(x, scale),

			Self::FocusElement(x) => smithay::backend::renderer::element::Element::opaque_regions(x, scale),
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}

	fn alpha(&self) -> f32 {
		match self {
			Self::DefaultPointer(x) => smithay::backend::renderer::element::Element::alpha(x),

			Self::CropSurface(x) => smithay::backend::renderer::element::Element::alpha(x),

			Self::Surface(x) => smithay::backend::renderer::element::Element::alpha(x),

			Self::FocusElement(x) => smithay::backend::renderer::element::Element::alpha(x),
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}

	fn kind(&self) -> smithay::backend::renderer::element::Kind {
		match self {
			Self::DefaultPointer(x) => smithay::backend::renderer::element::Element::kind(x),

			Self::CropSurface(x) => smithay::backend::renderer::element::Element::kind(x),

			Self::Surface(x) => smithay::backend::renderer::element::Element::kind(x),

			Self::FocusElement(x) => smithay::backend::renderer::element::Element::kind(x),
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}
}

impl RenderElement<GlowRenderer> for OutputRenderElements<GlowRenderer> {
	fn draw(
		&self,
		frame: &mut <GlowRenderer as smithay::backend::renderer::Renderer>::Frame<'_>,
		src: smithay::utils::Rectangle<f64, smithay::utils::Buffer>,
		dst: smithay::utils::Rectangle<i32, smithay::utils::Physical>,
		damage: &[smithay::utils::Rectangle<i32, smithay::utils::Physical>],
		opaque_regions: &[smithay::utils::Rectangle<i32, smithay::utils::Physical>],
	) -> Result<(), <GlowRenderer as smithay::backend::renderer::Renderer>::Error> {
		match self {
			Self::DefaultPointer(x) => x.draw(frame, src, dst, damage, opaque_regions),
			Self::CropSurface(x) => x.draw(frame, src, dst, damage, opaque_regions),
			Self::Surface(x) => x.draw(frame, src, dst, damage, opaque_regions),
			Self::FocusElement(x) => {
				RenderElement::<GlowRenderer>::draw(x, frame, src, dst, damage, opaque_regions)
			}
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}
	#[inline]
	fn underlying_storage(
		&self,
		renderer: &mut GlowRenderer,
	) -> Option<smithay::backend::renderer::element::UnderlyingStorage<'_>> {
		match self {
			Self::DefaultPointer(x) => x.underlying_storage(renderer),
			Self::CropSurface(x) => x.underlying_storage(renderer),
			Self::Surface(x) => x.underlying_storage(renderer),

			Self::FocusElement(x) => x.underlying_storage(renderer),
			Self::_GenericCatcher(_) => unreachable!(),
		}
	}
}
impl<R> From<MemoryRenderBufferRenderElement<R>> for OutputRenderElements<R>
where
	R: smithay::backend::renderer::Renderer,
{
	#[inline]
	fn from(field: MemoryRenderBufferRenderElement<R>) -> OutputRenderElements<R> {
		OutputRenderElements::DefaultPointer(field)
	}
}

impl<R> From<CropRenderElement<WaylandSurfaceRenderElement<R>>> for OutputRenderElements<R>
where
	R: smithay::backend::renderer::Renderer,
{
	#[inline]
	fn from(field: CropRenderElement<WaylandSurfaceRenderElement<R>>) -> OutputRenderElements<R> {
		OutputRenderElements::CropSurface(field)
	}
}

impl<R> From<WaylandSurfaceRenderElement<R>> for OutputRenderElements<R>
where
	R: smithay::backend::renderer::Renderer,
{
	#[inline]
	fn from(field: WaylandSurfaceRenderElement<R>) -> OutputRenderElements<R> {
		OutputRenderElements::Surface(field)
	}
}

impl<R> From<PixelShaderElement> for OutputRenderElements<R>
where
	R: smithay::backend::renderer::Renderer,
{
	#[inline]
	fn from(field: PixelShaderElement) -> OutputRenderElements<R> {
		OutputRenderElements::FocusElement(field)
	}
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
			OutputRenderElements::FocusElement(element) => {
				f.debug_tuple("FocusElement").field(&element).finish()
			}
			OutputRenderElements::_GenericCatcher(_) => f.write_str("_GenericCatcher"),
		}
	}
}
