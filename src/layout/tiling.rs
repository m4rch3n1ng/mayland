use crate::{render::OutputRenderElements, shell::window::MappedWindow, utils::output_size};
use smithay::{
	backend::renderer::{element::AsRenderElements, glow::GlowRenderer},
	desktop::WindowSurface,
	output::Output,
	utils::{Logical, Point, Rectangle, Size},
};

#[derive(Debug)]
struct LayoutSize {
	rect: Rectangle<i32, Logical>,
	ratio: f64,

	gaps: i32,
}

impl LayoutSize {
	fn new() -> Self {
		let size = Size::from((0, 0));

		// todo border
		let loc = Point::from((25, 25));
		let rect = Rectangle { loc, size };

		let ratio = 0.5;

		let gaps = 20;

		LayoutSize { rect, ratio, gaps }
	}

	fn resize(&mut self, size: Size<i32, Logical>) {
		self.rect.size = size;
	}

	fn split(&self) -> Point<i32, Logical> {
		let x = self.rect.size.w as f64 * self.ratio;
		let x = x.round() as i32;

		// todo border
		Point::from((x, 25))
	}

	fn single(&self) -> Rectangle<i32, Logical> {
		self.rect
	}

	fn double(&self) -> (Rectangle<i32, Logical>, Rectangle<i32, Logical>) {
		let split = self.split();
		let size = self.rect.size;

		let one = {
			let size = Size::from((size.w - split.x - self.gaps / 2, size.h));
			let loc = self.rect.loc;
			Rectangle { loc, size }
		};

		let two = {
			let size = Size::from((size.w - one.size.w, size.h));
			let loc = Point::from((split.x + self.gaps / 2, split.y));
			Rectangle { loc, size }
		};

		(one, two)
	}
}

#[derive(Debug)]
struct Layout {
	layout: LayoutSize,

	border: i32,

	one: Option<MappedWindow>,
	two: Option<MappedWindow>,
}

impl Layout {
	fn new() -> Self {
		let size = LayoutSize::new();

		Layout {
			layout: size,
			border: 25,

			one: None,
			two: None,
		}
	}
}

impl Layout {
	fn map_output(&mut self, size: Size<i32, Logical>) {
		let size = layout_size(size, self.border);
		self.layout.resize(size);
	}

	fn resize_output(&mut self, output_size: Size<i32, Logical>) {
		let layout_size = layout_size(output_size, self.border);
		self.layout.resize(layout_size);

		self.layout.double();

		tracing::debug!("tiling window resize {:?}", layout_size);

		if let Some(one) = &self.one {
			match one.underlying_surface() {
				WindowSurface::Wayland(xdg) => {
					xdg.with_pending_state(|state| {
						state.size = Some(layout_size);
					});
					xdg.send_pending_configure();
				}
			}
		}
	}
}

impl Layout {
	fn window_location(&self, window: &MappedWindow) -> Point<i32, Logical> {
		if self.one.as_ref().is_some_and(|w| w == window) {
			let window_location = self.layout.single().loc;
			window.render_location(window_location)
		} else if self.two.as_ref().is_some_and(|w| w == window) {
			let window_location = self.layout.double().1.loc;
			window.render_location(window_location)
		} else {
			unreachable!()
		}
	}
}

impl Layout {
	fn add_window(&mut self, mapped: MappedWindow) -> Option<MappedWindow> {
		if self.one.is_none() {
			tracing::debug!("add tiling window");

			let size = self.layout.single().size;
			match mapped.window.underlying_surface() {
				WindowSurface::Wayland(xdg) => {
					xdg.with_pending_state(|state| {
						state.size = Some(size);
					});
					xdg.send_pending_configure();
				}
			}

			self.one = Some(mapped);
			None
		} else if self.two.is_none() {
			let (one, two) = self.layout.double();

			let window = self.one.as_ref().unwrap();
			match window.window.underlying_surface() {
				WindowSurface::Wayland(xdg) => {
					xdg.with_pending_state(|state| {
						state.size = Some(one.size);
					});
					xdg.send_pending_configure();
				}
			}

			match mapped.window.underlying_surface() {
				WindowSurface::Wayland(xdg) => {
					xdg.with_pending_state(|state| {
						state.size = Some(two.size);
					});
					xdg.send_pending_configure();
				}
			}

			self.two = Some(mapped);
			None
		} else {
			Some(mapped)
		}
	}

	fn remove_window(&mut self, window: &MappedWindow) -> bool {
		if self.one.as_ref().is_some_and(|current| current == window) {
			self.one = None;
			true
		} else {
			false
		}
	}

	fn has_window(&self, window: &MappedWindow) -> bool {
		self.windows().any(|w| w == window)
	}

	fn windows(&self) -> impl DoubleEndedIterator<Item = &MappedWindow> {
		self.one.iter().chain(self.two.iter())
	}

	fn window_under(&self, _location: Point<f64, Logical>) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		self.one
			.as_ref()
			.map(|window| (window, self.window_location(window)))
	}
}

impl Layout {
	fn render(&self, renderer: &mut GlowRenderer, scale: f64) -> Vec<OutputRenderElements<GlowRenderer>> {
		if let Some(window) = &self.one {
			let window_location = Point::from((self.border, self.border));

			let window_render_location = window
				.render_location(window_location)
				.to_physical_precise_round(scale);

			window.render_elements(renderer, window_render_location, scale.into(), 1.)
		} else {
			vec![]
		}
	}
}

#[derive(Debug)]
pub struct Tiling {
	size: Option<Size<i32, Logical>>,

	layout: Layout,
}

impl Tiling {
	#[allow(clippy::new_without_default)]
	pub fn new() -> Self {
		let layout = Layout::new();

		Tiling { size: None, layout }
	}
}

fn layout_size(size: Size<i32, Logical>, border: i32) -> Size<i32, Logical> {
	Size::from((
		size.w.saturating_sub(border * 2),
		size.h.saturating_sub(border * 2),
	))
}

impl Tiling {
	pub fn map_output(&mut self, output: &Output) {
		let output_size = output_size(output);

		self.size = Some(output_size);
		self.layout.map_output(output_size);
	}

	pub fn unmap_output(&mut self) {
		self.size = None;
	}

	pub fn resize_output(&mut self, size: Size<i32, Logical>) {
		self.size = Some(size);
		self.layout.resize_output(size);
	}
}

impl Tiling {
	/// add [`MappedWindow`] if the tiling space isn't full, otherwise return it again
	pub fn add_window(&mut self, mapped: MappedWindow) -> Option<MappedWindow> {
		self.layout.add_window(mapped)
	}

	/// removes a [`MappedWindow`] from the tiling space if it exists
	///
	/// returns `true` if a window was removed, `false` otherwise
	pub fn remove_window(&mut self, window: &MappedWindow) -> bool {
		self.layout.remove_window(window)
	}

	pub fn has_window(&self, window: &MappedWindow) -> bool {
		self.layout.has_window(window)
	}

	pub fn windows(&self) -> impl DoubleEndedIterator<Item = &MappedWindow> {
		self.layout.windows()
	}

	pub fn window_under(
		&self,
		location: Point<f64, Logical>,
	) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		self.layout.window_under(location)
	}
}

impl Tiling {
	pub fn render(&self, renderer: &mut GlowRenderer, scale: f64) -> Vec<OutputRenderElements<GlowRenderer>> {
		self.layout.render(renderer, scale)
	}
}
