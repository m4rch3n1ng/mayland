use crate::{render::MaylandRenderElements, shell::element::MappedWindowElement};
use smithay::{
	backend::renderer::glow::GlowRenderer,
	desktop::Space,
	output::Output,
	utils::{Logical, Point, Rectangle},
};

#[derive(Debug)]
pub struct WorkspaceManager {
	space: Space<MappedWindowElement>,
}

impl WorkspaceManager {
	pub fn new() -> Self {
		let space = Space::default();

		WorkspaceManager { space }
	}
}

impl WorkspaceManager {
	pub fn add_output(&mut self, output: Output) {
		let x = self
			.space
			.outputs()
			.map(|output| self.space.output_geometry(output).unwrap())
			.map(|geom| geom.loc.x + geom.size.w)
			.max()
			.unwrap_or(0);

		self.space.map_output(&output, (x, 0));
	}

	pub fn remove_output(&mut self, output: &Output) {
		self.space.unmap_output(output);
	}

	pub fn refresh(&mut self) {
		self.space.refresh();
	}
}

impl WorkspaceManager {
	pub fn outputs(&self) -> impl Iterator<Item = &Output> {
		self.space.outputs()
	}

	pub fn outputs_for_element(&self, element: &MappedWindowElement) -> Vec<Output> {
		self.space.outputs_for_element(element)
	}

	pub fn output_geometry(&self, output: &Output) -> Option<Rectangle<i32, Logical>> {
		self.space.output_geometry(output)
	}

	pub fn output_under<P: Into<Point<f64, Logical>>>(
		&self,
		point: P,
	) -> impl Iterator<Item = &Output> {
		self.space.output_under(point)
	}

	pub fn space_elements(
		&self,
		renderer: &mut GlowRenderer,
		output: &Output,
	) -> Vec<MaylandRenderElements> {
		let space_elements = smithay::desktop::space::space_render_elements::<
			_,
			MappedWindowElement,
			_,
		>(renderer, [&self.space], output, 1.0)
		.unwrap();

		space_elements
			.into_iter()
			.map(MaylandRenderElements::Space)
			.collect()
	}
}

impl WorkspaceManager {
	pub fn map_element<P: Into<Point<i32, Logical>>>(
		&mut self,
		element: MappedWindowElement,
		location: P,
		activate: bool,
	) {
		self.space.map_element(element, location, activate);
	}

	pub fn raise_element(&mut self, element: &MappedWindowElement, activate: bool) {
		self.space.raise_element(element, activate);
	}

	pub fn elements(&self) -> impl DoubleEndedIterator<Item = &MappedWindowElement> {
		self.space.elements()
	}

	pub fn element_location(&self, element: &MappedWindowElement) -> Option<Point<i32, Logical>> {
		self.space.element_location(element)
	}

	pub fn element_geometry(
		&self,
		element: &MappedWindowElement,
	) -> Option<Rectangle<i32, Logical>> {
		self.space.element_geometry(element)
	}

	pub fn element_under<P: Into<Point<f64, Logical>>>(
		&self,
		point: P,
	) -> Option<(&MappedWindowElement, Point<i32, Logical>)> {
		self.space.element_under(point)
	}
}
