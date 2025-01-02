use crate::{
	backend::{udev::Udev, winit::Winit, Backend},
	comm::MaySocket,
	cursor::{Cursor, RenderCursor},
	input::{apply_libinput_settings, device::InputDevice},
	layout::workspace::WorkspaceManager,
	render::MaylandRenderElements,
	shell::{focus::KeyboardFocusTarget, window::UnmappedSurface},
	utils::output_size,
};
use calloop::futures::Scheduler;
use indexmap::IndexSet;
use mayland_comm::MAYLAND_SOCKET_VAR;
use mayland_config::{bind::CompMod, Config};
use smithay::{
	backend::{
		input::Keycode,
		renderer::{
			element::{
				memory::MemoryRenderBufferRenderElement,
				solid::{SolidColorBuffer, SolidColorRenderElement},
				surface::render_elements_from_surface_tree,
				Kind, RenderElementStates,
			},
			glow::GlowRenderer,
		},
	},
	desktop::{
		layer_map_for_output,
		utils::{
			surface_presentation_feedback_flags_from_states, surface_primary_scanout_output,
			OutputPresentationFeedback,
		},
		LayerSurface, PopupManager,
	},
	input::{keyboard::KeyboardHandle, pointer::PointerHandle, Seat, SeatState},
	output::Output,
	reexports::{
		calloop::{generic::Generic, EventLoop, Interest, LoopHandle, LoopSignal, Mode, PostAction},
		wayland_server::{
			backend::{ClientData, GlobalId},
			Display, DisplayHandle,
		},
	},
	utils::{Clock, IsAlive, Monotonic},
	wayland::{
		compositor::{CompositorClientState, CompositorState},
		cursor_shape::CursorShapeManagerState,
		dmabuf::DmabufState,
		output::OutputManagerState,
		presentation::PresentationState,
		relative_pointer::RelativePointerManagerState,
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
		viewporter::ViewporterState,
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
	) -> Result<Self, mayland_config::Error> {
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
	) -> Result<Self, mayland_config::Error> {
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

	pub fn reload_config(&mut self, config: Config) {
		if self.mayland.config == config {
			return;
		}

		if self.mayland.config.input != config.input {
			for mut device in self.mayland.devices.iter().cloned() {
				apply_libinput_settings(&config.input, &mut device);
			}

			if self.mayland.config.input.keyboard != config.input.keyboard {
				let xkb_config = config.input.keyboard.xkb_config();

				let xkb = self.mayland.seat.get_keyboard().unwrap();
				xkb.set_xkb_config(self, xkb_config).unwrap();
				if let Some(xkb_file) = config.input.keyboard.xkb_file.as_deref() {
					let keymap = std::fs::read_to_string(xkb_file).unwrap();
					xkb.set_keymap_from_string(self, keymap).unwrap();
				}
			}
		}

		if self.mayland.config.decoration.background != config.decoration.background {
			for output_state in self.mayland.output_state.values_mut() {
				output_state.background.set_color(config.decoration.background);
			}
		}

		if self.mayland.config.decoration.focus != config.decoration.focus
			|| self.mayland.config.layout != config.layout
		{
			self.mayland.workspaces.reload_config(&config);
		}

		if self.mayland.config.windowrules != config.windowrules {
			for window in self.mayland.workspaces.windows() {
				window.recompute_windowrules(&config.windowrules);
			}
		}

		self.mayland.config = config;
		self.mayland.queue_redraw_all();
	}

	pub fn refresh_and_redraw(&mut self) {
		// refresh workspaces and popups
		self.mayland.workspaces.refresh();
		self.mayland.popups.cleanup();

		// redraw the queued outputs
		self.mayland.redraw_all_queued(&mut self.backend);
		self.mayland.display_handle.flush_clients().unwrap();

		// cleanup dead surfaces
		self.mayland.unmapped_windows.retain(|window| window.alive());
		self.mayland.unmapped_layers.retain(|(layer, _)| layer.alive());
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
	pub unmapped_layers: Vec<(LayerSurface, Output)>,

	pub start_time: std::time::Instant,
	pub loop_signal: LoopSignal,
	pub loop_handle: LoopHandle<'static, State>,
	pub scheduler: Scheduler<()>,

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
	pub relative_pointer_manager_state: RelativePointerManagerState,
	pub viewporter_state: ViewporterState,

	// input
	pub devices: IndexSet<InputDevice>,
	pub pointer: PointerHandle<State>,
	pub keyboard: KeyboardHandle<State>,
	pub cursor: Cursor,

	pub may_socket: MaySocket,

	pub comp_mod: CompMod,
	pub suppressed_keys: HashSet<Keycode>,
}

#[derive(Debug)]
pub struct OutputState {
	pub global: GlobalId,
	/// queued state
	pub queued: QueueState,
	/// use a solid color buffer instead of a clear color, so that
	/// the background color cuts out at the sides when mirroring
	/// outputs instead of filling the entire output
	///
	/// apparently it also avoids damage tracking issues
	pub background: SolidColorBuffer,
}

#[derive(Debug, Clone, Copy)]
pub enum QueueState {
	Idle,
	WaitingForVBlank { queued: bool },
	Queued,
}

impl QueueState {
	pub fn is_queued(&self) -> bool {
		matches!(self, QueueState::Queued)
	}

	pub fn queue(&mut self) {
		match *self {
			QueueState::Idle => *self = QueueState::Queued,
			QueueState::WaitingForVBlank { queued: false } => {
				*self = QueueState::WaitingForVBlank { queued: true }
			}
			QueueState::Queued | QueueState::WaitingForVBlank { queued: true } => {}
		}
	}

	pub fn idle(&mut self) {
		if let QueueState::Queued = *self {
			*self = QueueState::Idle
		} else {
			unreachable!()
		}
	}

	pub fn waiting_for_vblank(&mut self) {
		if let QueueState::Queued = *self {
			unreachable!()
		} else {
			*self = QueueState::WaitingForVBlank { queued: false };
		}
	}

	pub fn on_vblank(&mut self) {
		if let QueueState::WaitingForVBlank { queued } = *self {
			if queued {
				*self = QueueState::Queued;
			} else {
				*self = QueueState::Idle;
			}
		} else {
			unreachable!()
		}
	}
}

impl Mayland {
	fn new(
		event_loop: &mut EventLoop<'static, State>,
		display: Display<State>,
		comp_mod: CompMod,
	) -> Result<Self, mayland_config::Error> {
		let loop_handle = event_loop.handle();

		let (config, rx) = Config::init(comp_mod)?;
		loop_handle
			.insert_source(rx, |event, (), state| match event {
				calloop::channel::Event::Msg(config) => state.reload_config(config),
				calloop::channel::Event::Closed => (),
			})
			.unwrap();

		let mut environment = HashMap::new();

		let display_handle = display.handle();
		let socket_name = init_wayland_display(display, event_loop);

		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "winit");
		let clock = Clock::new();

		let popups = PopupManager::default();

		let workspaces = WorkspaceManager::new(&config);

		let start_time = Instant::now();
		let loop_signal = event_loop.get_signal();
		let (executor, scheduler) = calloop::futures::executor().unwrap();
		loop_handle.insert_source(executor, |(), (), _| ()).unwrap();

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
		let relative_pointer_manager_state = RelativePointerManagerState::new::<State>(&display_handle);
		let viewporter_state = ViewporterState::new::<State>(&display_handle);

		let devices = IndexSet::new();
		let keyboard = seat
			.add_keyboard(
				config.input.keyboard.xkb_config(),
				config.input.keyboard.repeat_delay,
				config.input.keyboard.repeat_rate,
			)
			.unwrap();
		let pointer = seat.add_pointer();
		let cursor = Cursor::new(&config.cursor, &mut environment);

		let may_socket = MaySocket::init(&loop_handle, &socket_name);
		environment.insert(
			MAYLAND_SOCKET_VAR.to_owned(),
			// todo fix this if it ever results in a panic
			may_socket.path.clone().into_os_string().into_string().unwrap(),
		);

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
			unmapped_layers: Vec::new(),

			start_time,
			loop_signal,
			loop_handle,
			scheduler,

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
			relative_pointer_manager_state,
			viewporter_state,

			devices,
			pointer,
			keyboard,
			cursor,

			may_socket,

			comp_mod,
			suppressed_keys: HashSet::new(),
		};

		Ok(mayland)
	}
}

impl Mayland {
	pub fn add_output(&mut self, output: Output) {
		tracing::debug!("add output {:?}", output.description());

		if let Some(relocate) = self.workspaces.add_output(&self.config.output, &output) {
			self.loop_handle.insert_idle(move |state| {
				state.relocate(relocate);
			});
		}

		let background_color = self.config.decoration.background;
		let size = output_size(&output);
		let state = OutputState {
			global: output.create_global::<State>(&self.display_handle),
			queued: QueueState::Idle,
			background: SolidColorBuffer::new(size, background_color),
		};

		let prev = self.output_state.insert(output, state);
		assert!(prev.is_none(), "output was already tracked");
	}

	pub fn remove_output(&mut self, output: &Output) {
		let state = self.output_state.remove(output).unwrap();
		self.display_handle.remove_global::<State>(state.global);

		if let Some(relocate) = self.workspaces.remove_output(&self.config.output, output) {
			self.loop_handle.insert_idle(move |state| {
				state.relocate(relocate);
			});
		}
	}

	/// only the working area of the output has changed
	///
	/// the output does not need to be remapped
	pub fn output_area_changed(&mut self, output: &Output) {
		layer_map_for_output(output).arrange();
		self.workspaces.output_area_changed(output);
	}

	/// the output changed actual size and needs to (potentially) be remapped
	pub fn output_size_changed(&mut self, output: &Output) {
		layer_map_for_output(output).arrange();
		self.workspaces.output_size_changed(&self.config.output, output);

		let size = output_size(output);
		let output_state = self.output_state.get_mut(output).unwrap();
		output_state.background.resize(size);
	}

	pub fn queue_redraw_all(&mut self) {
		for state in self.output_state.values_mut() {
			state.queued.queue();
		}
	}

	pub fn queue_redraw(&mut self, output: Output) {
		let output_state = self.output_state.get_mut(&output).unwrap();
		output_state.queued.queue();
	}

	fn redraw_all_queued(&mut self, backend: &mut Backend) {
		while let Some((output, _)) = self
			.output_state
			.iter()
			.find(|(_, state)| state.queued.is_queued())
		{
			let output = output.clone();
			self.redraw(backend, &output);
		}
	}

	fn redraw(&mut self, backend: &mut Backend, output: &Output) {
		let output_state = self.output_state.get_mut(output).unwrap();
		output_state.queued.idle();

		let renderer = backend.renderer();
		let elements = self.elements(renderer, output);

		backend.render(self, output, &elements);
		self.display_handle.flush_clients().unwrap();
	}

	fn elements(&mut self, renderer: &mut GlowRenderer, output: &Output) -> Vec<MaylandRenderElements> {
		let mut elements = Vec::new();

		let pointer_element = self.pointer_element(renderer, output);
		elements.extend(pointer_element);

		let focus = self.keyboard.current_focus().and_then(|focus| match focus {
			KeyboardFocusTarget::Window(mapped) => Some(mapped),
			_ => None,
		});

		let workspace_elements = self.workspaces.render_elements(renderer, output, focus);
		elements.extend(workspace_elements);

		let output_state = &self.output_state[output];
		elements.push(MaylandRenderElements::Solid(
			SolidColorRenderElement::from_buffer(
				&output_state.background,
				(0, 0),
				1.0,
				1.0,
				Kind::Unspecified,
			),
		));

		elements
	}

	fn pointer_element(
		&mut self,
		renderer: &mut GlowRenderer,
		output: &Output,
	) -> Vec<MaylandRenderElements> {
		let output_position = self.workspaces.output_position(output).unwrap();

		let pointer_pos = self.pointer.current_location() - output_position.to_f64();
		let pointer_pos = pointer_pos.to_physical(1.);

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

		for mapped in self.workspaces.windows_for_output(output) {
			mapped.window.take_presentation_feedback(
				&mut output_presentation_feedback,
				surface_primary_scanout_output,
				|surface, _| surface_presentation_feedback_flags_from_states(surface, render_element_states),
			);
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

	pub fn send_frame_callbacks(&self, output: &Output) {
		for mapped in self.workspaces.windows_for_output(output) {
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
	event_loop
		.handle()
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
