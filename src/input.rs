use crate::{
	action::Action,
	shell::{
		element::WindowElement,
		focus::{KeyboardFocusTarget, PointerFocusTarget},
	},
	state::State,
};
use smithay::{
	backend::{
		input::{
			AbsolutePositionEvent, Axis, AxisSource, Event, InputBackend, InputEvent, KeyState,
			KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
		},
		winit::WinitInput,
	},
	desktop::{layer_map_for_output, WindowSurfaceType},
	input::{
		keyboard::{FilterResult, KeyboardHandle, Keysym, KeysymHandle, ModifiersState},
		pointer::{AxisFrame, ButtonEvent, MotionEvent},
	},
	reexports::wayland_server::protocol::wl_pointer,
	utils::{Logical, Point, Serial, SERIAL_COUNTER},
	wayland::{input_method::InputMethodSeat, shell::wlr_layer::Layer as WlrLayer},
};

impl State {
	pub fn handle_input_event(&mut self, event: InputEvent<WinitInput>) {
		match event {
			InputEvent::Keyboard { event, .. } => {
				let keyboard = self.mayland.keyboard.clone();

				let code = event.key_code();
				let key_state = event.state();
				let serial = SERIAL_COUNTER.next_serial();
				let time = event.time_msec();

				let Some(Some(action)) = keyboard.input(
					self,
					code,
					key_state,
					serial,
					time,
					|state, mods, keysym| state.handle_key(code, key_state, mods, keysym),
				) else {
					return;
				};

				self.handle_action(action);
			}
			InputEvent::PointerMotion { .. } => {}
			InputEvent::PointerMotionAbsolute { event } => {
				self.on_pointer_move_absolute::<WinitInput>(event);
			}
			InputEvent::PointerButton { event } => self.on_pointer_button::<WinitInput>(event),
			InputEvent::PointerAxis { event } => self.on_pointer_axis::<WinitInput>(event),

			evt => println!("evt {:?}", evt),
		}
	}

	fn handle_key(
		&mut self,
		code: u32,
		key_state: KeyState,
		mods: &ModifiersState,
		keysym: KeysymHandle,
	) -> FilterResult<Option<Action>> {
		let Some(raw_sym) = keysym.raw_latin_sym_or_raw_current_sym() else {
			return FilterResult::Forward;
		};

		if key_state == KeyState::Released {
			if self.mayland.suppressed_keys.take(&code).is_some() {
				return FilterResult::Intercept(None);
			} else {
				return FilterResult::Forward;
			}
		};

		let action = if mods.alt && raw_sym == Keysym::Escape {
			Some(Action::Quit)
		} else if mods.alt && raw_sym == Keysym::q {
			Some(Action::CloseWindow)
		} else if mods.alt && raw_sym == Keysym::t {
			Some(Action::Spawn("alacritty".to_owned()))
		} else {
			None
		};

		if let Some(action) = action {
			self.mayland.suppressed_keys.insert(code);
			FilterResult::Intercept(Some(action))
		} else {
			FilterResult::Forward
		}
	}

	fn on_pointer_move_absolute<B: InputBackend>(&mut self, event: B::PointerMotionAbsoluteEvent) {
		let output = self.mayland.space.outputs().next().unwrap().clone();
		let output_geo = self.mayland.space.output_geometry(&output).unwrap();
		let location = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

		let under = self.surface_under(location);
		let serial = SERIAL_COUNTER.next_serial();

		self.update_keyboard_focus(location, serial);

		let ptr = self.mayland.pointer.clone();
		ptr.motion(
			self,
			under,
			&MotionEvent {
				location,
				serial,
				time: event.time_msec(),
			},
		);
		ptr.frame(self);
	}

	fn on_pointer_button<B: InputBackend>(&mut self, event: B::PointerButtonEvent) {
		let serial = SERIAL_COUNTER.next_serial();
		let button = event.button_code();
		let state = wl_pointer::ButtonState::from(event.state());

		if let wl_pointer::ButtonState::Pressed = state {
			self.update_keyboard_focus(self.mayland.pointer.current_location(), serial);
		}

		let pointer = self.mayland.pointer.clone();
		pointer.button(
			self,
			&ButtonEvent {
				button,
				state: state.try_into().unwrap(),
				serial,
				time: event.time_msec(),
			},
		);
		pointer.frame(self);
	}

	fn on_pointer_axis<B: InputBackend>(&mut self, event: B::PointerAxisEvent) {
		let horizontal_amount_v120 = event.amount_v120(Axis::Horizontal);
		let horizontal_amount = event
			.amount(Axis::Horizontal)
			.or_else(|| horizontal_amount_v120.map(|amt| amt * 15. / 120.))
			.unwrap_or(0.0);
		let vertical_amount_v120 = event.amount_v120(Axis::Vertical);
		let vertical_amount = event
			.amount(Axis::Vertical)
			.or_else(|| vertical_amount_v120.map(|amt| amt * 15. / 120.))
			.unwrap_or(0.0);

		let mut frame = AxisFrame::new(event.time_msec()).source(event.source());
		if horizontal_amount != 0.0 {
			frame = frame
				.relative_direction(Axis::Horizontal, event.relative_direction(Axis::Horizontal));
			frame = frame.value(Axis::Horizontal, horizontal_amount);
			if let Some(amount_v120) = horizontal_amount_v120 {
				frame = frame.v120(Axis::Horizontal, amount_v120 as i32);
			}
		}
		if vertical_amount != 0.0 {
			frame =
				frame.relative_direction(Axis::Vertical, event.relative_direction(Axis::Vertical));
			frame = frame.value(Axis::Vertical, vertical_amount);
			if let Some(amount_v120) = vertical_amount_v120 {
				frame = frame.v120(Axis::Vertical, amount_v120 as i32);
			}
		}
		if event.source() == AxisSource::Finger {
			if event.amount(Axis::Horizontal) == Some(0.0) {
				frame = frame.stop(Axis::Horizontal);
			}
			if event.amount(Axis::Vertical) == Some(0.0) {
				frame = frame.stop(Axis::Vertical);
			}
		}

		let pointer = self.mayland.pointer.clone();
		pointer.axis(self, frame);
		pointer.frame(self);
	}

	fn update_keyboard_focus(&mut self, location: Point<f64, Logical>, serial: Serial) {
		let keyboard = self.mayland.keyboard.clone();
		let input_method = self.mayland.seat.input_method();

		if self.mayland.pointer.is_grabbed()
			|| keyboard.is_grabbed() && !input_method.keyboard_grabbed()
		{
			return;
		}

		let output = self.mayland.space.output_under(location).next().cloned();
		if let Some(output) = output.as_ref() {
			let output_geo = self.mayland.space.output_geometry(output).unwrap();
			let layers = layer_map_for_output(output);
			if let Some(layer) = layers
				.layer_under(WlrLayer::Overlay, location)
				.or_else(|| layers.layer_under(WlrLayer::Top, location))
			{
				if layer.can_receive_keyboard_focus() {
					let layer_geo = layers.layer_geometry(layer).unwrap();
					if let Some((_, _)) = layer.surface_under(
						location - output_geo.loc.to_f64() - layer_geo.loc.to_f64(),
						WindowSurfaceType::ALL,
					) {
						keyboard.set_focus(self, Some(KeyboardFocusTarget::from(layer)), serial);
					}
				}
			}
		};

		if let Some((window, _)) = self
			.mayland
			.space
			.element_under(location)
			.map(|(w, p)| (w.clone(), p))
		{
			self.focus_window(window, &keyboard, serial);
		}

		if let Some(output) = output.as_ref() {
			let layers = layer_map_for_output(output);
			if let Some(layer) = layers
				.layer_under(WlrLayer::Bottom, location)
				.or_else(|| layers.layer_under(WlrLayer::Background, location))
			{
				if layer.can_receive_keyboard_focus() {
					let output_geo = self.mayland.space.output_geometry(output).unwrap();
					let layer_geo = layers.layer_geometry(layer).unwrap();
					if let Some((_, _)) = layer.surface_under(
						location - output_geo.loc.to_f64() - layer_geo.loc.to_f64(),
						WindowSurfaceType::ALL,
					) {
						keyboard.set_focus(self, Some(KeyboardFocusTarget::from(layer)), serial);
					}
				}
			}
		}
	}

	pub fn focus_window(
		&mut self,
		window: WindowElement,
		keyboard: &KeyboardHandle<State>,
		serial: Serial,
	) {
		self.mayland.space.raise_element(&window, true);
		keyboard.set_focus(self, Some(KeyboardFocusTarget::from(window)), serial);
		self.mayland.space.elements().for_each(|window| {
			window.0.toplevel().unwrap().send_pending_configure();
		});
	}

	pub fn surface_under(
		&self,
		location: Point<f64, Logical>,
	) -> Option<(PointerFocusTarget, Point<i32, Logical>)> {
		let output = self.mayland.space.outputs().find(|output| {
			let geometry = self.mayland.space.output_geometry(output).unwrap();
			geometry.contains(location.to_i32_round())
		})?;
		let output_geo = self.mayland.space.output_geometry(output).unwrap();
		let layers = layer_map_for_output(output);

		if let Some(layer) = layers
			.layer_under(WlrLayer::Overlay, location)
			.or_else(|| layers.layer_under(WlrLayer::Top, location))
		{
			let layer_loc = layers.layer_geometry(layer).unwrap();
			layer
				.surface_under(
					location - output_geo.loc.to_f64() - layer_loc.loc.to_f64(),
					WindowSurfaceType::ALL,
				)
				.map(|(surface, loc)| (PointerFocusTarget::from(surface), loc))
		} else if let Some((window, loc)) = self.mayland.space.element_under(location) {
			window
				.surface_under(location - loc.to_f64(), WindowSurfaceType::ALL)
				.map(|(surface, surf_loc)| (surface, surf_loc + loc))
		} else if let Some(layer) = layers
			.layer_under(WlrLayer::Bottom, location)
			.or_else(|| layers.layer_under(WlrLayer::Background, location))
		{
			let layer_loc = layers.layer_geometry(layer).unwrap();
			layer
				.surface_under(
					location - output_geo.loc.to_f64() - layer_loc.loc.to_f64(),
					WindowSurfaceType::ALL,
				)
				.map(|(surface, loc)| (PointerFocusTarget::from(surface), loc))
		} else {
			None
		}
	}
}
