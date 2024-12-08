use crate::{
	render::{FocusRing, MaylandRenderElements},
	shell::window::MappedWindow,
	utils::RectExt,
};
use smithay::{
	backend::renderer::glow::GlowRenderer,
	desktop::{layer_map_for_output, space::SpaceElement},
	output::Output,
	utils::{Logical, Point, Rectangle, Size},
};

type WindowLayout = (MappedWindow, Rectangle<i32, Logical>);

#[derive(Debug)]
struct Layout {
	/// the output working area, excluding layer-shell
	/// exclusive zones
	working_area: Rectangle<i32, Logical>,
	/// the area that i can actually map windows to
	useable_area: Rectangle<i32, Logical>,

	/// the border around the windows
	border: i32,
	/// the gaps between windows
	gaps: i32,
	/// thickness of the focus ring
	ring: i32,

	/// split point between 0 and 1
	ratio: f64,
	split: Point<i32, Logical>,
}

impl Layout {
	fn new(border: i32, gaps: i32, decoration: &mayland_config::Decoration) -> Self {
		let working_area = Rectangle::default();
		let useable_area = Rectangle {
			loc: Point::from((border, border)),
			size: Size::from((0, 0)),
		};

		Layout {
			working_area,
			useable_area,

			border,
			gaps,
			ring: i32::from(decoration.focus.thickness),

			ratio: 0.5,
			split: Point::from((0, 0)),
		}
	}

	fn resize(&mut self, working_area: Rectangle<i32, Logical>) {
		self.working_area = working_area;
		self.useable_area = working_area.borderless(self.border);

		let x = self.useable_area.size.w as f64 * self.ratio;
		let x = x.round() as i32;

		let rel = Point::from((x, 0));
		let split = self.useable_area.loc + rel;
		self.split = split;
	}

	fn single(&self) -> Rectangle<i32, Logical> {
		self.useable_area.borderless(self.ring)
	}

	fn double(&self) -> [Rectangle<i32, Logical>; 2] {
		let split = self.split;
		let rel_split = split - self.useable_area.loc;
		let size = self.useable_area.size;

		let gap = self.gaps / 2;

		let one = {
			let size = Size::from((rel_split.x - gap, size.h));
			let loc = self.useable_area.loc;

			Rectangle { loc, size }
		};

		let two = {
			let size = Size::from((size.w - one.size.w - self.gaps, size.h));
			let loc = Point::from((split.x + gap, split.y));

			Rectangle { loc, size }
		};

		[one.borderless(self.ring), two.borderless(self.ring)]
	}
}

enum Position {
	Left,
	Right,
}

impl Layout {
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
pub struct Tiling {
	layout: Layout,
	windows: [Option<WindowLayout>; 2],
}

impl Tiling {
	pub fn new(decoration: &mayland_config::Decoration) -> Self {
		let border = 20;
		let gaps = 10;

		let layout = Layout::new(border, gaps, decoration);

		Tiling {
			layout,
			windows: [None, None],
		}
	}
}

impl Tiling {
	pub fn add_window(&mut self, window: MappedWindow, pointer: Point<f64, Logical>) -> Option<MappedWindow> {
		for w in self.windows() {
			w.set_activate(false);
		}

		match &mut self.windows {
			[Some(_), Some(_)] => Some(window),
			[None, Some(_)] => unreachable!(),
			[Some(prev), empty @ None] => {
				let [one, two] = self.layout.double();
				let position = self.layout.position(pointer);

				window.set_activate(true);

				match position {
					Position::Left => {
						prev.0.resize(two.size);
						prev.1 = two;

						window.resize(one.size);
						*empty = Some((window, one));

						self.windows.swap(0, 1);
					}
					Position::Right => {
						prev.0.resize(one.size);
						prev.1 = one;

						window.resize(two.size);
						*empty = Some((window, two));
					}
				}

				None
			}
			[empty @ None, None] => {
				let one = self.layout.single();

				window.set_activate(true);
				window.resize(one.size);
				*empty = Some((window, one));

				None
			}
		}
	}

	pub fn remove_window(&mut self, window: &MappedWindow) -> bool {
		match &mut self.windows {
			[Some(w1), Some(w2)] if &w1.0 == window => {
				let one = self.layout.single();
				w2.0.resize(one.size);
				w2.1 = one;

				self.windows.swap(0, 1);
				self.windows[1] = None;

				true
			}
			[Some(w1), Some(w2)] if &w2.0 == window => {
				let one = self.layout.single();
				w1.0.resize(one.size);
				w1.1 = one;

				self.windows[1] = None;
				true
			}
			[None, Some(_)] => unreachable!(),
			[Some(prev), None] if &prev.0 == window => {
				self.windows[0] = None;
				true
			}
			_ => false,
		}
	}

	pub fn activate_window(&mut self, window: &MappedWindow) {
		for w in self.windows() {
			if w == window {
				w.set_activate(true);
			} else {
				w.set_activate(false);
			}
		}
	}

	pub fn map_output(&mut self, output: &Output) {
		let layout_size = layer_map_for_output(output).non_exclusive_zone();
		self.resize(layout_size);
	}

	pub fn resize_output(&mut self, output: &Output) {
		let layout_size = layer_map_for_output(output).non_exclusive_zone();
		self.resize(layout_size);
	}

	fn resize(&mut self, working_area: Rectangle<i32, Logical>) {
		self.layout.resize(working_area);

		match &mut self.windows {
			[Some(w1), Some(w2)] => {
				let [one, two] = self.layout.double();

				w1.0.resize(one.size);
				w1.1 = one;

				w2.0.resize(two.size);
				w2.1 = two;
			}
			[Some(window), None] => {
				let one = self.layout.single();
				window.0.resize(one.size);
				window.1 = one;
			}
			[None, Some(_)] => unreachable!(),
			[None, None] => (),
		}
	}
}

impl Tiling {
	pub fn has_window(&self, window: &MappedWindow) -> bool {
		self.windows().any(|w| w == window)
	}

	pub fn windows(&self) -> impl DoubleEndedIterator<Item = &MappedWindow> {
		self.windows.iter().flatten().map(|w| &w.0)
	}

	fn windows_geometry(&self) -> impl DoubleEndedIterator<Item = &(MappedWindow, Rectangle<i32, Logical>)> {
		self.windows.iter().flatten()
	}

	pub fn window_under(
		&self,
		location: Point<f64, Logical>,
	) -> Option<(&MappedWindow, Point<i32, Logical>)> {
		match &self.windows {
			[Some(w1), Some(w2)] => {
				let position = self.layout.position(location);
				match position {
					Position::Left => {
						let loc = w1.0.render_location(w1.1.loc);
						Some((&w1.0, loc))
					}
					Position::Right => {
						let loc = w2.0.render_location(w2.1.loc);
						Some((&w2.0, loc))
					}
				}
			}
			[Some(window), None] => {
				let loc = window.0.render_location(window.1.loc);
				Some((&window.0, loc))
			}
			[None, Some(_)] => unreachable!(),
			[None, None] => None,
		}
	}
}

impl Tiling {
	pub fn render<'a, 'b>(
		&self,
		renderer: &'a mut GlowRenderer,
		scale: f64,
		decoration: &'b mayland_config::Decoration,
		focus: Option<MappedWindow>,
	) -> impl Iterator<Item = MaylandRenderElements> + use<'_, 'a, 'b> {
		self.windows_geometry().flat_map(move |(window, geom)| {
			let window_rect = window.render_rectangle(*geom);

			let render_location = window_rect.to_physical_precise_round(scale);
			let mut elements = window.crop_render_elements(renderer, render_location, scale.into(), 1.);

			let color = if focus.as_ref().is_some_and(|focus| focus == window) {
				decoration.focus.active
			} else {
				decoration.focus.inactive
			};

			let focus_ring = FocusRing::element(renderer, *geom, color.as_f32s(), decoration.focus.thickness);
			elements.push(MaylandRenderElements::FocusElement(focus_ring));

			elements
		})
	}
}
