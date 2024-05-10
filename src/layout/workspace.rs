use crate::{render::MaylandRenderElements, shell::element::MappedWindowElement};
use smithay::{
	backend::renderer::glow::GlowRenderer,
	desktop::Space,
	output::Output,
	utils::{Logical, Point, Rectangle},
};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct WorkspaceManager {
	space: Space<MappedWindowElement>,

	workspaces: BTreeMap<usize, Workspace>,
	current: usize,
}

impl WorkspaceManager {
	pub fn new() -> Self {
		let space = Space::default();

		let workspace = Workspace::new();
		let workspaces = BTreeMap::from([(0, workspace)]);
		let current = 0;

		WorkspaceManager {
			space,
			workspaces,
			current,
		}
	}
}

impl WorkspaceManager {
	pub fn switch_to_workspace(&mut self, idx: usize) {
		if idx == self.current {
			return;
		}

		// todo use current output
		let output = self.space.outputs().next().unwrap();
		self.workspaces
			.get_mut(&self.current)
			.unwrap()
			.unmap_output(output);

		let workspace = self.workspaces.entry(idx).or_insert_with(Workspace::new);
		workspace.map_output(output);
		self.current = idx;
	}

	pub fn workspace(&self) -> &Workspace {
		&self.workspaces[&self.current]
	}

	pub fn workspace_mut(&mut self) -> &mut Workspace {
		self.workspaces.get_mut(&self.current).unwrap()
	}
}

impl WorkspaceManager {
	pub fn add_output(&mut self, output: &Output) {
		let x = self
			.space
			.outputs()
			.map(|output| self.space.output_geometry(output).unwrap())
			.map(|geom| geom.loc.x + geom.size.w)
			.max()
			.unwrap_or(0);

		self.space.map_output(output, (x, 0));

		let workspace = self.workspace_mut();
		workspace.map_output(output);
	}

	pub fn remove_output(&mut self, output: &Output) {
		self.space.unmap_output(output);

		let workspace = self.workspace_mut();
		workspace.unmap_output(output);
	}

	pub fn refresh(&mut self) {
		self.space.refresh();

		let workspace = self.workspace_mut();
		workspace.refresh();
	}

	pub fn render_elements(
		&self,
		renderer: &mut GlowRenderer,
		output: &Output,
	) -> impl Iterator<Item = MaylandRenderElements> {
		let workspace = self.workspace();
		workspace.space_elements(renderer, output)
	}
}

impl WorkspaceManager {
	pub fn outputs(&self) -> impl Iterator<Item = &Output> {
		self.space.outputs()
	}

	pub fn outputs_for_element(&self, element: &MappedWindowElement) -> Vec<Output> {
		let workspace = self.workspace();
		workspace.outputs_for_element(element)
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
}

impl WorkspaceManager {
	pub fn map_element<P: Into<Point<i32, Logical>>>(
		&mut self,
		element: MappedWindowElement,
		location: P,
		activate: bool,
	) {
		let workspace = self.workspace_mut();
		workspace.map_element(element, location, activate);
	}

	pub fn raise_element(&mut self, element: &MappedWindowElement, activate: bool) {
		let workspace = self.workspace_mut();
		workspace.raise_element(element, activate);
	}

	pub fn elements(&self) -> impl DoubleEndedIterator<Item = &MappedWindowElement> {
		let workspace = self.workspace();
		workspace.elements()
	}

	pub fn element_location(&self, element: &MappedWindowElement) -> Option<Point<i32, Logical>> {
		let workspace = self.workspace();
		workspace.element_location(element)
	}

	pub fn element_geometry(
		&self,
		element: &MappedWindowElement,
	) -> Option<Rectangle<i32, Logical>> {
		let workspace = self.workspace();
		workspace.element_geometry(element)
	}

	pub fn element_under<P: Into<Point<f64, Logical>>>(
		&self,
		point: P,
	) -> Option<(&MappedWindowElement, Point<i32, Logical>)> {
		let workspace = self.workspace();
		workspace.element_under(point)
	}
}

impl Default for WorkspaceManager {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug)]
pub struct Workspace {
	space: Space<MappedWindowElement>,
}

impl Workspace {
	fn new() -> Self {
		let space = Space::default();

		Workspace { space }
	}
}

impl Workspace {
	fn map_output(&mut self, output: &Output) {
		self.space.map_output(output, (0, 0));
	}

	fn unmap_output(&mut self, output: &Output) {
		self.space.unmap_output(output);
	}

	fn outputs_for_element(&self, element: &MappedWindowElement) -> Vec<Output> {
		self.space.outputs_for_element(element)
	}

	fn refresh(&mut self) {
		self.space.refresh();
	}

	pub fn is_empty(&self) -> bool {
		self.elements().count() == 0
	}
}

impl Workspace {
	fn space_elements(
		&self,
		renderer: &mut GlowRenderer,
		output: &Output,
	) -> impl Iterator<Item = MaylandRenderElements> {
		let space_elements = smithay::desktop::space::space_render_elements::<
			_,
			MappedWindowElement,
			_,
		>(renderer, [&self.space], output, 1.0)
		.unwrap();

		space_elements.into_iter().map(MaylandRenderElements::Space)
	}
}

impl Workspace {
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
