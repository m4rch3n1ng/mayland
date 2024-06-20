use crate::{
	render::OutputRenderElements,
	shell::window::MappedWindow,
	utils::{output_size, SizeExt},
};
use smithay::{
	backend::renderer::{element::AsRenderElements, glow::GlowRenderer},
	output::Output,
	utils::{Logical, Point, Rectangle, Size},
};

enum Position {
	Left,
	Right,
}

#[derive(Debug)]
struct LayoutSize {
	loc: Point<i32, Logical>,
	size: Size<i32, Logical>,

	gaps: i32,

	ratio: f64,

	split: Point<i32, Logical>,
	single: Rectangle<i32, Logical>,
	double: (Rectangle<i32, Logical>, Rectangle<i32, Logical>),
}

impl LayoutSize {
	fn new() -> Self {
		let border = 25;
		let gaps = 20;

		let loc = Point::from((border, border));
		let size = Size::from((0, 0));

		let ratio = 0.5;

		let split = loc;
		let single = Rectangle { loc, size };
		let double = (single, single);

		LayoutSize {
			loc,
			size,

			gaps,

			ratio,

			split,
			single,
			double,
		}
	}

	fn resize(&mut self, size: Size<i32, Logical>) {
		self.size = size;

		// todo maybe inline
		self.set_split();
		self.set_single();
		self.set_double();
	}

	fn set_split(&mut self) {
		let x = self.size.w as f64 * self.ratio;
		let x = x.round() as i32;

		let rel = Point::from((x, 0));
		let split = self.loc + rel;
		self.split = split;
	}

	fn set_single(&mut self) {
		self.single = Rectangle {
			loc: self.loc,
			size: self.size,
		};
	}

	fn set_double(&mut self) {
		let split = self.split;
		let rel_split = split - self.loc;
		let size = self.size;

		let gap = self.gaps / 2;

		let one = {
			let size = Size::from((rel_split.x - gap, size.h));
			let loc = self.loc;
			Rectangle { loc, size }
		};

		let two = {
			let size = Size::from((size.w - one.size.w - self.gaps, size.h));
			let loc = Point::from((split.x + gap, split.y));
			Rectangle { loc, size }
		};

		self.double = (one, two);
	}
}

impl LayoutSize {
	fn position(&self, location: Point<f64, Logical>) -> Position {
		let split = self.split.to_f64();
		if location.x <= split.x {
			Position::Left
		} else {
			Position::Right
		}
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
		let size = size.borderless(self.border);
		self.layout.resize(size);
	}

	fn resize_windows(&self) {
		match (&self.one, &self.two) {
			(Some(window1), Some(window2)) => {
				let (one, two) = self.layout.double;

				window1.resize(one.size);
				window2.resize(two.size);
			}
			(Some(window), None) => {
				let layout = self.layout.single;
				window.resize(layout.size);
			}
			(None, Some(_)) => unreachable!(),
			(None, None) => {}
		}
	}

	fn resize_output(&mut self, output_size: Size<i32, Logical>) {
		let layout_size = output_size.borderless(self.border);

		tracing::debug!("tiling window resize {:?}", layout_size);

		self.layout.resize(layout_size);
		self.resize_windows();
	}
}

impl Layout {
	fn add_window(&mut self, mapped: MappedWindow, pointer: Point<f64, Logical>) -> Option<MappedWindow> {
		if let Some(window) = &self.one {
			if self.two.is_none() {
				let (one, two) = self.layout.double;

				let position = self.layout.position(pointer);
				if let Position::Left = position {
					window.resize(two.size);
					mapped.resize(one.size);

					let prev = self.one.replace(mapped);
					self.two = prev;
				} else {
					window.resize(one.size);
					mapped.resize(two.size);

					self.two = Some(mapped);
				}

				None
			} else {
				Some(mapped)
			}
		} else {
			tracing::debug!("add tiling window");

			let size = self.layout.single.size;
			mapped.resize(size);

			self.one = Some(mapped);
			None
		}
	}

	fn remove_window(&mut self, window: &MappedWindow) -> bool {
		if self.one.as_ref().is_some_and(|current| current == window) {
			self.one = self.two.take();
			self.resize_windows();

			true
		} else if self.two.as_ref().is_some_and(|current| current == window) {
			self.two = None;
			self.resize_windows();

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

	fn window_under(&self, location: Point<f64, Logical>) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		match (&self.one, &self.two) {
			(Some(window1), Some(window2)) => {
				let position = self.layout.position(location);
				match position {
					Position::Left => {
						let window_location = self.layout.double.0.loc;
						let window_location = window1.render_location(window_location);
						Some((window1, window_location))
					}
					Position::Right => {
						let window_location = self.layout.double.1.loc;
						let window_location = window2.render_location(window_location);
						Some((window2, window_location))
					}
				}
			}
			(Some(window), None) => {
				let window_location = self.layout.single.loc;
				let window_location = window.render_location(window_location);
				Some((window, window_location))
			}
			(None, Some(_)) => unreachable!(),
			(None, None) => None,
		}
	}
}

impl Layout {
	fn render(&self, renderer: &mut GlowRenderer, scale: f64) -> Vec<OutputRenderElements<GlowRenderer>> {
		match (&self.one, &self.two) {
			(Some(window1), Some(window2)) => {
				let window_layout = self.layout.double;

				let mut render = {
					let window_location = window_layout.0.loc;

					let window_render_location = window1
						.render_location(window_location)
						.to_physical_precise_round(scale);

					window1.render_elements(renderer, window_render_location, scale.into(), 1.)
				};

				render.extend({
					let window_location = window_layout.1.loc;

					let window_render_location = window2
						.render_location(window_location)
						.to_physical_precise_round(scale);

					window2.render_elements(renderer, window_render_location, scale.into(), 1.)
				});

				render
			}
			(Some(window), None) => {
				let window_location = self.layout.single.loc;

				let window_render_location = window
					.render_location(window_location)
					.to_physical_precise_round(scale);

				window.render_elements(renderer, window_render_location, scale.into(), 1.)
			}
			(None, Some(_)) => unreachable!(),
			(None, None) => vec![],
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
	pub fn add_window(&mut self, mapped: MappedWindow, pointer: Point<f64, Logical>) -> Option<MappedWindow> {
		self.layout.add_window(mapped, pointer)
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
