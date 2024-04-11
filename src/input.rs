use crate::{shell::focus::PointerFocusTarget, state::State};
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
	wayland::shell::wlr_layer::Layer as WlrLayer,
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

			_ => {}
		}
	}

	fn on_pointer_move_absolute<B: InputBackend>(&mut self, event: B::PointerMotionAbsoluteEvent) {
		let output = self.space.outputs().next().unwrap().clone();
		let output_geo = self.space.output_geometry(&output).unwrap();
		let location = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

		let under = self.surface_under(location);

		let ptr = self.pointer.clone();
		let serial = SERIAL_COUNTER.next_serial();

		self.update_keyboard_focus(location, serial);

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

		if let Some((window, _)) = self
			.space
			.element_under(location)
			.map(|(w, p)| (w.clone(), p))
		{
			self.space.raise_element(&window, true);
			keyboard.set_focus(self, Some(window.into()), serial);
		}
	}

	pub fn surface_under(
		&self,
		pos: Point<f64, Logical>,
	) -> Option<(PointerFocusTarget, Point<i32, Logical>)> {
		let output = self.space.outputs().find(|output| {
			let geometry = self.space.output_geometry(output).unwrap();
			geometry.contains(pos.to_i32_round())
		})?;
		let output_geo = self.space.output_geometry(output).unwrap();
		let layers = layer_map_for_output(output);

		if let Some(layer) = layers
			.layer_under(WlrLayer::Overlay, pos)
			.or_else(|| layers.layer_under(WlrLayer::Top, pos))
		{
			let layer_loc = layers.layer_geometry(layer).unwrap().loc;
			layer
				.surface_under(
					pos - output_geo.loc.to_f64() - layer_loc.to_f64(),
					WindowSurfaceType::ALL,
				)
				.map(|(surface, loc)| (PointerFocusTarget::from(surface), loc))
		} else if let Some((window, loc)) = self.space.element_under(pos) {
			window
				.surface_under(pos - loc.to_f64(), WindowSurfaceType::ALL)
				.map(|(surface, surf_loc)| (surface, surf_loc + loc))
		} else {
			None
		}
	}
}
