use self::{element::MappedWindowElement, xdg::handle_commit};
use crate::state::{ClientState, State};
use smithay::{
	backend::renderer::utils::on_commit_buffer_handler,
	delegate_compositor, delegate_shm,
	desktop::{layer_map_for_output, LayerSurface},
	output::Output,
	reexports::wayland_server::{
		protocol::{wl_buffer, wl_output::WlOutput, wl_surface::WlSurface},
		Client,
	},
	wayland::{
		buffer::BufferHandler,
		compositor::{
			get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler,
			CompositorState,
		},
		seat::WaylandFocus,
		shell::wlr_layer::{
			Layer, LayerSurface as WlrLayerSurface, WlrLayerShellHandler, WlrLayerShellState,
		},
		shm::{ShmHandler, ShmState},
	},
};

pub mod element;
pub mod focus;
pub mod grab;
pub mod xdg;

impl BufferHandler for State {
	fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl CompositorHandler for State {
	fn compositor_state(&mut self) -> &mut CompositorState {
		&mut self.mayland.compositor_state
	}

	fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
		if let Some(state) = client.get_data::<ClientState>() {
			return &state.compositor_state;
		}

		panic!("unknown client data type")
	}

	fn commit(&mut self, surface: &WlSurface) {
		on_commit_buffer_handler::<Self>(surface);
		if !is_sync_subsurface(surface) {
			let mut root = surface.clone();
			while let Some(parent) = get_parent(&root) {
				root = parent;
			}

			if let Some(element) = self.element_for_surface(surface) {
				element.window.on_commit();
				self.mayland.queue_redraw_all();
			}
		};

		handle_commit(&mut self.mayland.popups, &self.mayland.space, surface);
		self.mayland.queue_redraw_all();
	}
}

impl WlrLayerShellHandler for State {
	fn shell_state(&mut self) -> &mut WlrLayerShellState {
		&mut self.mayland.layer_shell_state
	}

	fn new_layer_surface(
		&mut self,
		surface: WlrLayerSurface,
		wl_output: Option<WlOutput>,
		_layer: Layer,
		namespace: String,
	) {
		let output = wl_output
			.as_ref()
			.and_then(Output::from_resource)
			.unwrap_or_else(|| self.mayland.space.outputs().next().unwrap().clone());
		let mut map = layer_map_for_output(&output);
		map.map_layer(&LayerSurface::new(surface, namespace))
			.unwrap();
	}
}

impl State {
	fn element_for_surface(&mut self, surface: &WlSurface) -> Option<MappedWindowElement> {
		self.mayland
			.space
			.elements()
			.find(|&w| w.wl_surface().is_some_and(|w| w == *surface))
			.cloned()
	}
}

impl ShmHandler for State {
	fn shm_state(&self) -> &ShmState {
		&self.mayland.shm_state
	}
}

delegate_compositor!(State);
delegate_shm!(State);
