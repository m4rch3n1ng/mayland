use crate::{
	shell::focus::{KeyboardFocusTarget, PointerFocusTarget},
	state::State,
};
use smithay::{
	backend::{
		input::{
			AbsolutePositionEvent, Event, InputBackend, InputEvent, KeyboardKeyEvent,
			PointerButtonEvent,
		},
		winit::WinitInput,
	},
	desktop::{layer_map_for_output, WindowSurfaceType},
	input::{
		keyboard::FilterResult,
		pointer::{ButtonEvent, MotionEvent},
	},
	reexports::wayland_server::protocol::wl_pointer,
	utils::{Logical, Point, Serial, SERIAL_COUNTER},
	wayland::{input_method::InputMethodSeat, shell::wlr_layer::Layer as WlrLayer},
};

impl State {
	pub fn handle_input_event(&mut self, event: InputEvent<WinitInput>) {
		match event {
			InputEvent::Keyboard { event, .. } => {
				let keyboard = self.keyboard.clone();

				let code = event.key_code();
				let state = event.state();
				let serial = SERIAL_COUNTER.next_serial();
				let time = event.time_msec();

				let _ = keyboard.input(self, code, state, serial, time, |_state, _mods, keysym| {
					let raw_sym = keysym.raw_syms()[0];
					println!("key {:?}", raw_sym);

					FilterResult::Forward::<()>
				});
			}
			InputEvent::PointerMotion { .. } => {}
			InputEvent::PointerMotionAbsolute { event } => {
				self.on_pointer_move_absolute::<WinitInput>(event)
			}
			InputEvent::PointerButton { event } => self.on_pointer_button::<WinitInput>(event),

			evt => println!("evt {:?}", evt),
		}
	}

	fn on_pointer_move_absolute<B: InputBackend>(&mut self, event: B::PointerMotionAbsoluteEvent) {
		let output = self.space.outputs().next().unwrap().clone();
		let output_geo = self.space.output_geometry(&output).unwrap();
		let location = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

		let under = self.surface_under(location);
		let serial = SERIAL_COUNTER.next_serial();

		self.update_keyboard_focus(location, serial);

		let ptr = self.pointer.clone();
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
			self.update_keyboard_focus(self.pointer.current_location(), serial);
		}

		let pointer = self.pointer.clone();
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

	fn update_keyboard_focus(&mut self, location: Point<f64, Logical>, serial: Serial) {
		let keyboard = self.keyboard.clone();
		let input_method = self.seat.input_method();

		if self.pointer.is_grabbed() || keyboard.is_grabbed() && !input_method.keyboard_grabbed() {
			return;
		}

		let output = self.space.output_under(location).next().cloned();
		if let Some(output) = output.as_ref() {
			let output_geo = self.space.output_geometry(output).unwrap();
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
			.space
			.element_under(location)
			.map(|(w, p)| (w.clone(), p))
		{
			self.space.raise_element(&window, true);
			keyboard.set_focus(self, Some(KeyboardFocusTarget::from(window)), serial);
			self.space.elements().for_each(|window| {
				window.0.toplevel().unwrap().send_pending_configure();
			});
		}

		if let Some(output) = output.as_ref() {
			let layers = layer_map_for_output(output);
			if let Some(layer) = layers
				.layer_under(WlrLayer::Bottom, location)
				.or_else(|| layers.layer_under(WlrLayer::Background, location))
			{
				if layer.can_receive_keyboard_focus() {
					let output_geo = self.space.output_geometry(output).unwrap();
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

	pub fn surface_under(
		&self,
		location: Point<f64, Logical>,
	) -> Option<(PointerFocusTarget, Point<i32, Logical>)> {
		let output = self.space.outputs().find(|output| {
			let geometry = self.space.output_geometry(output).unwrap();
			geometry.contains(location.to_i32_round())
		})?;
		let output_geo = self.space.output_geometry(output).unwrap();
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
		} else if let Some((window, loc)) = self.space.element_under(location) {
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
