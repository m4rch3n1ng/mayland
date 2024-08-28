use crate::{
	backend::{udev::Udev, winit::Winit, Backend},
	cursor::{Cursor, RenderCursor},
	error::MaylandError,
	layout::workspace::WorkspaceManager,
	shell::window::UnmappedSurface,
};
use mayland_config::{bind::CompMod, Config};
use smithay::{
	backend::renderer::{
		element::{
			memory::MemoryRenderBufferRenderElement,
			surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
			Kind, RenderElementStates,
		},
		glow::GlowRenderer,
		ImportAll, ImportMem,
	},
	desktop::{
		layer_map_for_output,
		utils::{
			surface_presentation_feedback_flags_from_states, surface_primary_scanout_output,
			OutputPresentationFeedback,
		},
		PopupManager,
	},
	input::{keyboard::KeyboardHandle, pointer::PointerHandle, Seat, SeatState},
	output::Output,
	reexports::{
		calloop::{generic::Generic, EventLoop, Idle, Interest, LoopHandle, LoopSignal, Mode, PostAction},
		wayland_server::{
			backend::{ClientData, GlobalId},
			Display, DisplayHandle,
		},
	},
	render_elements,
	utils::{Clock, Monotonic},
	wayland::{
		compositor::{CompositorClientState, CompositorState},
		cursor_shape::CursorShapeManagerState,
		dmabuf::DmabufState,
		output::OutputManagerState,
		presentation::PresentationState,
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
	fmt::Debug,
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
	pub fn new_winit(
		event_loop: &mut EventLoop<'static, State>,
		display: Display<State>,
	) -> Result<Self, MaylandError> {
		let mut mayland = Mayland::new(event_loop, display, CompMod::Alt)?;

		let winit = Winit::init(&mut mayland);
		let winit = Backend::Winit(winit);

		Ok(State {
			backend: winit,
			mayland,
		})
	}

	pub fn new_udev(
		event_loop: &mut EventLoop<'static, State>,
		display: Display<State>,
	) -> Result<Self, MaylandError> {
		let mut mayland = Mayland::new(event_loop, display, CompMod::Meta)?;

		let udev = Udev::init(&mut mayland);
		let udev = Backend::Udev(udev);

		Ok(State {
			backend: udev,
			mayland,
		})
	}
}

impl State {
	pub fn load_config(&mut self) {
		if let Some(xkb_file) = self.mayland.config.input.keyboard.xkb_file.as_deref() {
			let keymap = std::fs::read_to_string(xkb_file).unwrap();

			let xkb = self.mayland.seat.get_keyboard().unwrap();
			xkb.set_keymap_from_string(self, keymap).unwrap();
		}
	}
}

#[derive(Debug)]
pub struct Mayland {
	pub config: Config,
	pub environment: HashMap<String, String>,

	pub display_handle: DisplayHandle,
	pub socket_name: String,

	pub seat: Seat<State>,
	pub popups: PopupManager,
	pub output_state: HashMap<Output, OutputState>,
	pub clock: Clock<Monotonic>,

	// workspace
	pub workspaces: WorkspaceManager,

	// unmapped_windows
	pub unmapped_windows: Vec<UnmappedSurface>,

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
	pub presentation_state: PresentationState,
	pub shm_state: ShmState,
	pub cursor_shape_manager_state: CursorShapeManagerState,

	// input
	pub pointer: PointerHandle<State>,
	pub keyboard: KeyboardHandle<State>,
	pub cursor: Cursor,

	pub suppressed_keys: HashSet<u32>,
}

#[derive(Debug)]
pub struct OutputState {
	pub global: GlobalId,
	pub queued: Option<Idle<'static>>,
	pub waiting_for_vblank: bool,
}

impl Mayland {
	fn new(
		event_loop: &mut EventLoop<'static, State>,
		display: Display<State>,
		comp: CompMod,
	) -> Result<Self, MaylandError> {
		let config = Config::read(comp)?;
		let mut environment = HashMap::new();

		let display_handle = display.handle();
		let socket_name = init_wayland_display(display, event_loop);

		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "winit");
		let clock = Clock::new();

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
		let presentation_state = PresentationState::new::<State>(&display_handle, clock.id() as u32);
		let shm_state = ShmState::new::<State>(&display_handle, vec![]);
		let cursor_shape_manager_state = CursorShapeManagerState::new::<State>(&display_handle);

		let keyboard = seat
			.add_keyboard(
				config.input.keyboard.xkb_config(),
				config.input.keyboard.repeat_delay,
				config.input.keyboard.repeat_rate,
			)
			.unwrap();
		let pointer = seat.add_pointer();
		let cursor = Cursor::new(&mut environment);

		let suppressed_keys = HashSet::new();

		let mayland = Mayland {
			config,
			environment,

			display_handle,
			socket_name,

			seat,
			popups,
			output_state: HashMap::new(),
			clock,

			workspaces,

			unmapped_windows: Vec::new(),

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
			presentation_state,
			shm_state,
			cursor_shape_manager_state,

			pointer,
			keyboard,
			cursor,

			suppressed_keys,
		};

		Ok(mayland)
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
		self.display_handle.flush_clients().unwrap();
	}

	fn elements(&mut self, renderer: &mut GlowRenderer, output: &Output) -> Vec<MaylandRenderElements> {
		let mut elements = Vec::new();

		if self.workspaces.is_active_output(output) {
			let pointer_element = self.pointer_element(renderer);
			elements.extend(pointer_element);
		}

		let workspace_elements = self.workspaces.render_elements(renderer, output);
		elements.extend(workspace_elements);

		elements
	}

	fn pointer_element(&mut self, renderer: &mut GlowRenderer) -> Vec<MaylandRenderElements> {
		let pointer_pos = self.workspaces.relative_cursor_location(&self.pointer);

		let render_cursor = self.cursor.get_render_cursor(1);
		match render_cursor {
			RenderCursor::Hidden => vec![],
			RenderCursor::Surface { surface, hotspot } => {
				let pointer_pos = pointer_pos.to_i32_round() - hotspot.to_physical(1);

				render_elements_from_surface_tree(renderer, &surface, pointer_pos, 1., 1., Kind::Cursor)
			}
			RenderCursor::Named(xcursor) => {
				let frame = xcursor.frame(self.start_time.elapsed());

				let hotspot = frame.hotspot();
				let buffer = frame.buffer();

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

				let render_element = MaylandRenderElements::DefaultPointer(texture);
				vec![render_element]
			}
		}
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

pub type MaylandRenderElements = OutputRenderElements<GlowRenderer>;

render_elements! {
	pub OutputRenderElements<R> where R: ImportAll + ImportMem;
	DefaultPointer = MemoryRenderBufferRenderElement<R>,
	Surface = WaylandSurfaceRenderElement<R>,
}

impl<R: ImportAll + ImportMem> Debug for OutputRenderElements<R> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			OutputRenderElements::DefaultPointer(element) => {
				f.debug_tuple("DefaultPointer").field(&element).finish()
			}
			OutputRenderElements::Surface(surface) => f.debug_tuple("Surface").field(&surface).finish(),
			OutputRenderElements::_GenericCatcher(_) => f.write_str("_GenericCatcher"),
		}
	}
}
