use crate::state::{ClientState, State};
use smithay::{
	backend::renderer::utils::on_commit_buffer_handler,
	delegate_compositor, delegate_layer_shell, delegate_shm,
	desktop::{LayerSurface, layer_map_for_output},
	output::Output,
	reexports::wayland_server::{
		Client,
		protocol::{wl_buffer, wl_output::WlOutput, wl_surface::WlSurface},
	},
	wayland::{
		buffer::BufferHandler,
		compositor::{
			CompositorClientState, CompositorHandler, CompositorState, get_parent, is_sync_subsurface,
		},
		shell::wlr_layer::{
			Layer, LayerSurface as WlrLayerSurface, WlrLayerShellHandler, WlrLayerShellState,
		},
		shm::{ShmHandler, ShmState},
	},
};

pub mod focus;
pub mod grab;
pub mod window;
pub mod wlr;
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

		if is_sync_subsurface(surface) {
			return;
		}

		let mut root = surface.clone();
		while let Some(parent) = get_parent(&root) {
			root = parent;
		}

		if surface == &root {
			self.try_map_window(&root);
			self.try_map_layer(&root);
		}

		// handle xdg surface commits
		self.mayland.handle_surface_commit(surface);
		// handle wlr layer shell surface commits
		self.mayland.handle_layer_surface_commit(surface);

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
		let Some(output) = (wl_output.as_ref().and_then(Output::from_resource))
			.or_else(|| self.mayland.workspaces.active_output().cloned())
		else {
			return;
		};

		let surface = LayerSurface::new(surface, namespace);
		self.mayland.unmapped_layers.push((surface, output));
	}

	fn layer_destroyed(&mut self, surface: WlrLayerSurface) {
		let output = self.mayland.workspaces.outputs().find_map(|output| {
			let mut layer_map = layer_map_for_output(output);
			let layer = layer_map
				.layers()
				.find(|layer| layer.layer_surface() == &surface)
				.cloned()?;

			layer_map.unmap_layer(&layer);
			Some(output.clone())
		});

		if let Some(output) = output {
			self.mayland.output_area_changed(&output);
			self.mayland.queue_redraw(output);
		}

		self.reset_focus();
	}
}

impl ShmHandler for State {
	fn shm_state(&self) -> &ShmState {
		&self.mayland.shm_state
	}
}

delegate_layer_shell!(State);
delegate_compositor!(State);
delegate_shm!(State);
