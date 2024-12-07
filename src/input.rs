use self::device::InputDevice;
use crate::{
	shell::{
		focus::{KeyboardFocusTarget, PointerFocusTarget},
		window::MappedWindow,
	},
	state::State,
};
use mayland_comm::Action;
use smithay::{
	backend::input::{
		AbsolutePositionEvent, Axis, AxisSource, Event, InputBackend, InputEvent, KeyState, KeyboardKeyEvent,
		Keycode, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
	},
	desktop::{layer_map_for_output, WindowSurfaceType},
	input::{
		keyboard::{
			keysyms::{KEY_XF86Switch_VT_1, KEY_XF86Switch_VT_12},
			FilterResult, KeyboardHandle, KeysymHandle, ModifiersState,
		},
		pointer::{AxisFrame, ButtonEvent, MotionEvent, RelativeMotionEvent},
	},
	reexports::wayland_server::protocol::wl_pointer,
	utils::{Logical, Point, Serial, SERIAL_COUNTER},
	wayland::{input_method::InputMethodSeat, shell::wlr_layer::Layer as WlrLayer},
};

pub mod device;

impl State {
	pub fn handle_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
		match event {
			InputEvent::DeviceAdded { .. } | InputEvent::DeviceRemoved { .. } => (),

			InputEvent::Keyboard { event, .. } => self.on_keyboard::<I>(event),
			InputEvent::PointerMotion { event } => self.on_pointer_move::<I>(event),
			InputEvent::PointerMotionAbsolute { event } => {
				self.on_pointer_move_absolute::<I>(event);
			}
			InputEvent::PointerButton { event } => self.on_pointer_button::<I>(event),
			InputEvent::PointerAxis { event } => self.on_pointer_axis::<I>(event),

			InputEvent::GestureSwipeBegin { .. } => (),
			InputEvent::GestureSwipeUpdate { .. } => (),
			InputEvent::GestureSwipeEnd { .. } => (),

			InputEvent::GesturePinchBegin { .. } => (),
			InputEvent::GesturePinchUpdate { .. } => (),
			InputEvent::GesturePinchEnd { .. } => (),

			InputEvent::GestureHoldBegin { .. } => (),
			InputEvent::GestureHoldEnd { .. } => (),

			InputEvent::TouchDown { .. } => tracing::info!("touch down"),
			InputEvent::TouchMotion { .. } => tracing::info!("touch motion"),
			InputEvent::TouchUp { .. } => tracing::info!("touch up"),
			InputEvent::TouchCancel { .. } => tracing::info!("touch cancel"),
			InputEvent::TouchFrame { .. } => tracing::info!("touch frame"),

			InputEvent::TabletToolAxis { .. } => tracing::info!("tablet tool axis"),
			InputEvent::TabletToolProximity { .. } => tracing::info!("tablet tool proximity"),
			InputEvent::TabletToolTip { .. } => tracing::info!("tablet tool tip"),
			InputEvent::TabletToolButton { .. } => tracing::info!("tablet tool button"),

			InputEvent::SwitchToggle { .. } => tracing::info!("switch toggle"),
			InputEvent::Special(_) => tracing::info!("special"),
		}
	}

	fn on_keyboard<I: InputBackend>(&mut self, event: I::KeyboardKeyEvent) {
		let keyboard = self.mayland.keyboard.clone();

		let code = event.key_code();
		let key_state = event.state();
		let serial = SERIAL_COUNTER.next_serial();
		let time = event.time_msec();

		let Some(Some(action)) =
			keyboard.input(self, code, key_state, serial, time, |state, mods, keysym| {
				state.handle_key(code, key_state, mods, keysym)
			})
		else {
			return;
		};

		self.handle_action(action);
	}

	fn on_pointer_move<I: InputBackend>(&mut self, event: I::PointerMotionEvent) {
		let pointer = self.mayland.pointer.clone();

		let mut location = pointer.current_location();
		location += event.delta();

		let mut min_max_y = None::<(i32, i32)>;
		let mut min_max_x = None::<(i32, i32)>;

		for output in self.mayland.workspaces.outputs() {
			let geom = self.mayland.workspaces.output_geometry(output).unwrap();
			min_max_y = min_max_y
				.map(|(min, max)| (i32::min(min, geom.loc.y), i32::max(max, geom.loc.y + geom.size.h)))
				.or(Some((geom.loc.y, geom.loc.y + geom.size.h)));
			min_max_x = min_max_x
				.map(|(min, max)| (i32::min(min, geom.loc.x), i32::max(max, geom.loc.x + geom.size.w)))
				.or(Some((geom.loc.x, geom.loc.x + geom.size.w)));
		}

		if let Some(((min_y, max_y), (min_x, max_x))) = min_max_y.zip(min_max_x) {
			location.y = location.y.clamp(f64::from(min_y), f64::from(max_y));
			location.x = location.x.clamp(f64::from(min_x), f64::from(max_x));
		}

		let under = self.surface_under(location);
		let serial = SERIAL_COUNTER.next_serial();

		if self.mayland.workspaces.update_active_output(location) {
			let workspace = self.mayland.workspaces.workspace();
			if workspace.is_none_or(|ws| ws.is_empty()) {
				let keyboard = self.mayland.keyboard.clone();
				keyboard.set_focus(self, None, serial);
			} else {
				self.update_keyboard_focus(location, serial);
			}
		} else {
			self.update_keyboard_focus(location, serial);
		}

		pointer.motion(
			self,
			under.clone(),
			&MotionEvent {
				location,
				serial,
				time: event.time_msec(),
			},
		);

		pointer.relative_motion(
			self,
			under,
			&RelativeMotionEvent {
				delta: event.delta(),
				delta_unaccel: event.delta_unaccel(),
				utime: event.time(),
			},
		);

		pointer.frame(self);

		self.mayland.queue_redraw_all();
	}

	fn on_pointer_move_absolute<I: InputBackend>(&mut self, event: I::PointerMotionAbsoluteEvent) {
		let output = self.mayland.workspaces.outputs().next().unwrap().clone();
		let output_geo = self.mayland.workspaces.output_geometry(&output).unwrap();
		let location = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

		let under = self.surface_under(location);
		let serial = SERIAL_COUNTER.next_serial();

		self.update_keyboard_focus(location, serial);

		let pointer = self.mayland.pointer.clone();
		pointer.motion(
			self,
			under,
			&MotionEvent {
				location,
				serial,
				time: event.time_msec(),
			},
		);
		pointer.frame(self);

		self.mayland.queue_redraw_all();
	}

	fn on_pointer_button<I: InputBackend>(&mut self, event: I::PointerButtonEvent) {
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

	fn on_pointer_axis<I: InputBackend>(&mut self, event: I::PointerAxisEvent) {
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
			frame = frame.relative_direction(Axis::Horizontal, event.relative_direction(Axis::Horizontal));
			frame = frame.value(Axis::Horizontal, horizontal_amount);
			if let Some(amount_v120) = horizontal_amount_v120 {
				frame = frame.v120(Axis::Horizontal, amount_v120 as i32);
			}
		}
		if vertical_amount != 0.0 {
			frame = frame.relative_direction(Axis::Vertical, event.relative_direction(Axis::Vertical));
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

	fn handle_key(
		&mut self,
		code: Keycode,
		key_state: KeyState,
		mods: &ModifiersState,
		keysym: KeysymHandle,
	) -> FilterResult<Option<Action>> {
		if let vt_key @ KEY_XF86Switch_VT_1..=KEY_XF86Switch_VT_12 = keysym.modified_sym().raw() {
			let vt = (vt_key - KEY_XF86Switch_VT_1 + 1) as i32;

			self.backend.switch_vt(vt);
			self.mayland.suppressed_keys.clear();

			return FilterResult::Intercept(None);
		}

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

		let action = self.mayland.config.bind.find_action(mods, raw_sym);

		if let Some(action) = action {
			self.mayland.suppressed_keys.insert(code);
			FilterResult::Intercept(Some(action))
		} else {
			FilterResult::Forward
		}
	}

	pub fn update_keyboard_focus(&mut self, location: Point<f64, Logical>, serial: Serial) {
		let keyboard = self.mayland.keyboard.clone();
		let input_method = self.mayland.seat.input_method();

		if self.mayland.pointer.is_grabbed() || keyboard.is_grabbed() && !input_method.keyboard_grabbed() {
			return;
		}

		let output = self.mayland.workspaces.output_under(location).next().cloned();
		if let Some(output) = output.as_ref() {
			let output_geo = self.mayland.workspaces.output_geometry(output).unwrap();
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
			.workspaces
			.window_under(location)
			.map(|(w, p)| (w.clone(), p))
		{
			self.set_window_focus(window, &keyboard, serial);
		}

		if let Some(output) = output.as_ref() {
			let layers = layer_map_for_output(output);
			if let Some(layer) = layers
				.layer_under(WlrLayer::Bottom, location)
				.or_else(|| layers.layer_under(WlrLayer::Background, location))
			{
				if layer.can_receive_keyboard_focus() {
					let output_geo = self.mayland.workspaces.output_geometry(output).unwrap();
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

		self.mayland.queue_redraw_all();
	}

	fn set_window_focus(&mut self, window: MappedWindow, keyboard: &KeyboardHandle<State>, serial: Serial) {
		self.mayland.workspaces.activate_window(&window);
		keyboard.set_focus(self, Some(KeyboardFocusTarget::from(window)), serial);
	}

	pub fn focus_window(&mut self, window: MappedWindow) {
		let serial = SERIAL_COUNTER.next_serial();
		let keyboard = self.mayland.keyboard.clone();
		self.set_window_focus(window, &keyboard, serial);
		self.refresh_pointer_focus();
	}

	pub fn surface_under(
		&self,
		location: Point<f64, Logical>,
	) -> Option<(PointerFocusTarget, Point<f64, Logical>)> {
		let output = self.mayland.workspaces.output_under(location).next()?;

		let output_geo = self.mayland.workspaces.output_geometry(output).unwrap();
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
				.map(|(surface, loc)| (PointerFocusTarget::from(surface), loc.to_f64()))
		} else if let Some((window, loc)) = self.mayland.workspaces.window_under(location) {
			Some((PointerFocusTarget::from(window), loc.to_f64()))
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
				.map(|(surface, loc)| (PointerFocusTarget::from(surface), loc.to_f64()))
		} else {
			None
		}
	}
}

pub fn apply_libinput_settings(config: &mayland_config::Input, device: &mut InputDevice) {
	if device.is_touchpad() {
		let conf = &config.touchpad;
		let device = &mut device.handle;

		let _ = device.config_tap_set_enabled(conf.tap);
		let _ = device.config_tap_set_drag_enabled(conf.tap_and_drag);
		let _ = device.config_tap_set_drag_lock_enabled(conf.tap_drag_lock);

		let _ = device.config_dwt_set_enabled(conf.dwt);
		let _ = device.config_dwtp_set_enabled(conf.dwtp);

		let _ = device.config_scroll_set_natural_scroll_enabled(conf.natural_scroll);
		if let Some(scroll_method) = conf.scroll_method {
			let _ = device.config_scroll_set_method(scroll_method);
		} else if let Some(default_scroll_method) = device.config_scroll_default_method() {
			let _ = device.config_scroll_set_method(default_scroll_method);
		}

		if let Some(click_method) = conf.click_method {
			let _ = device.config_click_set_method(click_method);
		} else if let Some(default_click_method) = device.config_click_default_method() {
			let _ = device.config_click_set_method(default_click_method);
		}

		let _ = device.config_middle_emulation_set_enabled(conf.middle_emulation);
		if let Some(tap_button_map) = conf.tap_button_map {
			let _ = device.config_tap_set_button_map(tap_button_map);
		} else if let Some(default_tap_button_map) = device.config_tap_default_button_map() {
			let _ = device.config_tap_set_button_map(default_tap_button_map);
		}
		let _ = device.config_left_handed_set(conf.left_handed);

		let accel_speed = conf.accel_speed.clamp(-1., 1.);
		if accel_speed != conf.accel_speed {
			tracing::warn!(
				"invalid accel_speed {}, clamping to {}",
				conf.accel_speed,
				accel_speed
			);
		}
		let _ = device.config_accel_set_speed(accel_speed);
		if let Some(accel_profile) = conf.accel_profile {
			let _ = device.config_accel_set_profile(accel_profile);
		} else if let Some(default_accel_profile) = device.config_accel_default_profile() {
			let _ = device.config_accel_set_profile(default_accel_profile);
		}
	} else if device.is_mouse() {
		let conf = &config.mouse;
		let device = &mut device.handle;

		let _ = device.config_scroll_set_natural_scroll_enabled(conf.natural_scroll);

		let _ = device.config_middle_emulation_set_enabled(conf.middle_emulation);
		let _ = device.config_left_handed_set(conf.left_handed);

		let accel_speed = conf.accel_speed.clamp(-1., 1.);
		if accel_speed != conf.accel_speed {
			tracing::warn!(
				"invalid accel speed {}, clamping to {}",
				conf.accel_speed,
				accel_speed
			);
		}
		let _ = device.config_accel_set_speed(accel_speed);
		if let Some(accel_profile) = conf.accel_profile {
			let _ = device.config_accel_set_profile(accel_profile);
		} else if let Some(default_accel_profile) = device.config_accel_default_profile() {
			let _ = device.config_accel_set_profile(default_accel_profile);
		}
	}
}
