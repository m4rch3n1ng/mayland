use crate::{
	backend::{udev::Udev, winit::Winit, Backend},
	layout::workspace::WorkspaceManager,
	render::{CursorBuffer, MaylandRenderElements},
};
use smithay::{
	backend::renderer::{
		element::{memory::MemoryRenderBufferRenderElement, Kind, RenderElementStates},
		glow::GlowRenderer,
	},
	desktop::{
		layer_map_for_output,
		utils::{
			surface_presentation_feedback_flags_from_states, surface_primary_scanout_output,
			OutputPresentationFeedback,
		},
		PopupManager, Window,
	},
	input::{
		keyboard::{KeyboardHandle, XkbConfig},
		pointer::{CursorImageStatus, PointerHandle},
		Seat, SeatState,
	},
	output::Output,
	reexports::{
		calloop::{generic::Generic, EventLoop, Idle, Interest, LoopHandle, LoopSignal, Mode, PostAction},
		wayland_server::{
			backend::{ClientData, GlobalId},
			protocol::wl_surface::WlSurface,
			Display, DisplayHandle,
		},
	},
	wayland::{
		compositor::{CompositorClientState, CompositorState},
		dmabuf::DmabufState,
		output::OutputManagerState,
		selection::{
			data_device::DataDeviceState, primary_selection::PrimarySelectionState,
			wlr_data_control::DataControlState,
		},
		shell::{
			wlr_layer::WlrLayerShellState,
			xdg::{decoration::XdgDecorationState, XdgShellState},
		},
		shm::ShmState,
		socket::ListeningSocketSource,
	},
};
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
	time::{Duration, Instant},
};

mod handlers;
mod pointer;

pub struct State {
	pub backend: Backend,
	pub mayland: Mayland,
}

impl State {
	pub fn new_winit(event_loop: &mut EventLoop<'static, State>, display: Display<State>) -> Self {
		let mut mayland = Mayland::new(event_loop, display);

		let winit = Winit::init(&mut mayland);
		let winit = Backend::Winit(winit);

		State {
			backend: winit,
			mayland,
		}
	}

	pub fn new_udev(event_loop: &mut EventLoop<'static, State>, display: Display<State>) -> Self {
		let mut mayland = Mayland::new(event_loop, display);

		let udev = Udev::init(&mut mayland);
		let udev = Backend::Udev(udev);

		State {
			backend: udev,
			mayland,
		}
	}
}

#[derive(Debug)]
pub struct Mayland {
	pub display_handle: DisplayHandle,
	pub socket_name: String,

	pub seat: Seat<State>,
	pub popups: PopupManager,
	pub output_state: HashMap<Output, OutputState>,

	// workspace
	pub workspaces: WorkspaceManager,

	// unmapped_windows
	pub unmapped_windows: HashMap<WlSurface, Window>,

	pub start_time: std::time::Instant,
	pub loop_signal: LoopSignal,
	pub loop_handle: LoopHandle<'static, State>,

	// wayland state
	pub compositor_state: CompositorState,
	pub data_device_state: DataDeviceState,
	pub dmabuf_state: DmabufState,
	pub layer_shell_state: WlrLayerShellState,
	pub output_manager_state: OutputManagerState,
	pub primary_selection_state: PrimarySelectionState,
	pub data_control_state: DataControlState,
	pub seat_state: SeatState<State>,
	pub xdg_decoration_state: XdgDecorationState,
	pub xdg_shell_state: XdgShellState,
	pub shm_state: ShmState,

	// input
	pub pointer: PointerHandle<State>,
	pub keyboard: KeyboardHandle<State>,
	pub cursor_image: CursorImageStatus,
	pub cursor_buffer: CursorBuffer,

	pub suppressed_keys: HashSet<u32>,
}

#[derive(Debug)]
pub struct OutputState {
	pub global: GlobalId,
	pub queued: Option<Idle<'static>>,
	pub waiting_for_vblank: bool,
}

impl Mayland {
	fn new(event_loop: &mut EventLoop<'static, State>, display: Display<State>) -> Self {
		let display_handle = display.handle();
		let socket_name = init_wayland_display(display, event_loop);

		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "winit");

		let popups = PopupManager::default();

		let workspaces = WorkspaceManager::new();

		let start_time = Instant::now();
		let loop_signal = event_loop.get_signal();
		let loop_handle = event_loop.handle();

		let compositor_state = CompositorState::new::<State>(&display_handle);
		let data_device_state = DataDeviceState::new::<State>(&display_handle);
		let dmabuf_state = DmabufState::new();
		let layer_shell_state = WlrLayerShellState::new::<State>(&display_handle);
		let output_manager_state = OutputManagerState::new_with_xdg_output::<State>(&display_handle);
		let primary_selection_state = PrimarySelectionState::new::<State>(&display_handle);
		let data_control_state =
			DataControlState::new::<State, _>(&display_handle, Some(&primary_selection_state), |_| true);
		let xdg_decoration_state = XdgDecorationState::new::<State>(&display_handle);
		let xdg_shell_state = XdgShellState::new::<State>(&display_handle);
		let shm_state = ShmState::new::<State>(&display_handle, vec![]);

		let keyboard = seat.add_keyboard(XkbConfig::default(), 200, 25).unwrap();
		let pointer = seat.add_pointer();
		let cursor_buffer = CursorBuffer::new();

		let suppressed_keys = HashSet::new();

		Mayland {
			display_handle,
			socket_name,

			seat,
			popups,
			output_state: HashMap::new(),

			workspaces,

			unmapped_windows: HashMap::new(),

			start_time,
			loop_signal,
			loop_handle,

			compositor_state,
			data_device_state,
			dmabuf_state,
			layer_shell_state,
			output_manager_state,
			primary_selection_state,
			data_control_state,
			seat_state,
			xdg_decoration_state,
			xdg_shell_state,
			shm_state,

			pointer,
			keyboard,
			cursor_image: CursorImageStatus::default_named(),
			cursor_buffer,

			suppressed_keys,
		}
	}
}

impl Mayland {
	pub fn add_output(&mut self, output: Output) {
		tracing::debug!("add output {:?}", output.description());

		self.workspaces.add_output(&output);

		let state = OutputState {
			global: output.create_global::<State>(&self.display_handle),
			queued: None,
			waiting_for_vblank: false,
		};

		let prev = self.output_state.insert(output, state);
		assert!(prev.is_none(), "output was already tracked");
	}

	pub fn remove_output(&mut self, output: &Output) {
		let mut state = self.output_state.remove(output).unwrap();
		self.display_handle.remove_global::<State>(state.global);

		if let Some(idle) = state.queued.take() {
			idle.cancel();
		};

		self.workspaces.remove_output(output);
	}

	pub fn output_resized(&mut self, output: &Output) {
		layer_map_for_output(output).arrange();
		self.workspaces.resize_output(output);
	}

	pub fn queue_redraw_all(&mut self) {
		let outputs = self.output_state.keys().cloned().collect::<Vec<_>>();
		for output in outputs {
			self.queue_redraw(output);
		}
	}

	pub fn queue_redraw(&mut self, output: Output) {
		let output_state = self.output_state.get_mut(&output).unwrap();

		if output_state.queued.is_some() || output_state.waiting_for_vblank {
			return;
		}

		let idle = self.loop_handle.insert_idle(move |state| {
			state.mayland.redraw(&mut state.backend, &output);
		});
		output_state.queued = Some(idle);
	}

	fn redraw(&mut self, backend: &mut Backend, output: &Output) {
		let output_state = self.output_state.get_mut(output).unwrap();

		assert!(output_state.queued.take().is_some());
		assert!(!output_state.waiting_for_vblank);

		let renderer = backend.renderer();
		let elements = self.elements(renderer, output);

		backend.render(self, output, &elements);
	}

	fn elements(&mut self, renderer: &mut GlowRenderer, output: &Output) -> Vec<MaylandRenderElements> {
		let mut elements = Vec::new();

		if self.workspaces.is_active_output(output) {
			let pointer_element = self.pointer_element(renderer);
			elements.push(pointer_element);
		}

		let workspace_elements = self.workspaces.render_elements(renderer, output);
		elements.extend(workspace_elements);

		elements
	}

	fn pointer_element(&mut self, renderer: &mut GlowRenderer) -> MaylandRenderElements {
		let pointer_pos = self.workspaces.relative_cursor_location(&self.pointer);

		let (buffer, hotspot) = self.cursor_buffer.get();
		let pointer_pos = pointer_pos - hotspot.to_f64();

		let texture = MemoryRenderBufferRenderElement::from_buffer(
			renderer,
			pointer_pos,
			buffer,
			None,
			None,
			None,
			Kind::Cursor,
		)
		.unwrap();

		MaylandRenderElements::DefaultPointer(texture)
	}

	pub fn presentation_feedback(
		&self,
		output: &Output,
		render_element_states: &RenderElementStates,
	) -> OutputPresentationFeedback {
		let mut output_presentation_feedback = OutputPresentationFeedback::new(output);

		for mapped in self.workspaces.windows() {
			if self.workspaces.outputs_for_window(mapped).contains(output) {
				mapped.window.take_presentation_feedback(
					&mut output_presentation_feedback,
					surface_primary_scanout_output,
					|surface, _| {
						surface_presentation_feedback_flags_from_states(surface, render_element_states)
					},
				);
			}
		}

		let layer_map = layer_map_for_output(output);
		for layer_surface in layer_map.layers() {
			layer_surface.take_presentation_feedback(
				&mut output_presentation_feedback,
				surface_primary_scanout_output,
				|surface, _| surface_presentation_feedback_flags_from_states(surface, render_element_states),
			);
		}

		output_presentation_feedback
	}

	pub fn post_repaint(&self, output: &Output) {
		for mapped in self.workspaces.windows() {
			mapped
				.window
				.send_frame(output, self.start_time.elapsed(), Some(Duration::ZERO), |_, _| {
					Some(output.clone())
				});
		}

		let layer_map = layer_map_for_output(output);
		for layer_surface in layer_map.layers() {
			layer_surface.send_frame(output, self.start_time.elapsed(), Some(Duration::ZERO), |_, _| {
				Some(output.clone())
			});
		}
	}
}

fn init_wayland_display(display: Display<State>, event_loop: &mut EventLoop<State>) -> String {
	// create socket for clients to connect to
	let source = ListeningSocketSource::new_auto().unwrap();
	let socket_name = source.socket_name().to_os_string().into_string().unwrap();

	let handle = event_loop.handle();

	event_loop
		.handle()
		.insert_source(source, move |client_stream, (), state| {
			// insert client into display
			state
				.mayland
				.display_handle
				.insert_client(client_stream, Arc::new(ClientState::default()))
				.unwrap();
		})
		.expect("failed to init the wayland event source.");

	// add display to event loop
	handle
		.insert_source(
			Generic::new(display, Interest::READ, Mode::Level),
			|_, display, state| {
				// SAFETY: we won't drop the display
				unsafe { display.get_mut().dispatch_clients(state).unwrap() };
				Ok(PostAction::Continue)
			},
		)
		.unwrap();

	socket_name
}

#[derive(Default)]
pub struct ClientState {
	pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {}
