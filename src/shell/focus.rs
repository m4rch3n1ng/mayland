use super::window::MappedWindow;
use crate::state::State;
use smithay::{
	backend::input::KeyState,
	desktop::{LayerSurface, PopupKind, WindowSurface},
	input::{
		keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
		pointer::{
			AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
			GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
			GestureSwipeUpdateEvent, MotionEvent, PointerTarget, RelativeMotionEvent,
		},
		Seat,
	},
	reexports::wayland_server::{backend::ObjectId, protocol::wl_surface::WlSurface, Resource},
	utils::{IsAlive, Serial},
	wayland::seat::WaylandFocus,
};
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub enum KeyboardFocusTarget {
	Window(MappedWindow),
	LayerSurface(LayerSurface),
	Popup(PopupKind),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerFocusTarget {
	WlSurface(WlSurface),
	Window(MappedWindow),
}

impl IsAlive for KeyboardFocusTarget {
	fn alive(&self) -> bool {
		match self {
			KeyboardFocusTarget::Window(w) => w.alive(),
			KeyboardFocusTarget::LayerSurface(l) => l.alive(),
			KeyboardFocusTarget::Popup(p) => p.alive(),
		}
	}
}

impl IsAlive for PointerFocusTarget {
	fn alive(&self) -> bool {
		match self {
			PointerFocusTarget::WlSurface(surface) => surface.alive(),
			PointerFocusTarget::Window(window) => window.alive(),
		}
	}
}

impl KeyboardTarget<State> for KeyboardFocusTarget {
	fn enter(&self, seat: &Seat<State>, data: &mut State, keys: Vec<KeysymHandle<'_>>, serial: Serial) {
		match self {
			KeyboardFocusTarget::Window(w) => match w.underlying_surface() {
				WindowSurface::Wayland(w) => {
					KeyboardTarget::enter(w.wl_surface(), seat, data, keys, serial);
				}
			},
			KeyboardFocusTarget::LayerSurface(l) => {
				KeyboardTarget::enter(l.wl_surface(), seat, data, keys, serial);
			}
			KeyboardFocusTarget::Popup(p) => {
				KeyboardTarget::enter(p.wl_surface(), seat, data, keys, serial);
			}
		}
	}

	fn leave(&self, seat: &Seat<State>, data: &mut State, serial: Serial) {
		match self {
			KeyboardFocusTarget::Window(w) => match w.underlying_surface() {
				WindowSurface::Wayland(w) => {
					KeyboardTarget::leave(w.wl_surface(), seat, data, serial);
				}
			},
			KeyboardFocusTarget::LayerSurface(l) => {
				KeyboardTarget::leave(l.wl_surface(), seat, data, serial);
			}
			KeyboardFocusTarget::Popup(p) => {
				KeyboardTarget::leave(p.wl_surface(), seat, data, serial);
			}
		}
	}

	fn key(
		&self,
		seat: &Seat<State>,
		data: &mut State,
		key: KeysymHandle<'_>,
		state: KeyState,
		serial: Serial,
		time: u32,
	) {
		match self {
			KeyboardFocusTarget::Window(w) => match w.underlying_surface() {
				WindowSurface::Wayland(w) => {
					KeyboardTarget::key(w.wl_surface(), seat, data, key, state, serial, time);
				}
			},
			KeyboardFocusTarget::LayerSurface(l) => {
				KeyboardTarget::key(l.wl_surface(), seat, data, key, state, serial, time);
			}
			KeyboardFocusTarget::Popup(p) => {
				KeyboardTarget::key(p.wl_surface(), seat, data, key, state, serial, time);
			}
		}
	}

	fn modifiers(&self, seat: &Seat<State>, data: &mut State, modifiers: ModifiersState, serial: Serial) {
		match self {
			KeyboardFocusTarget::Window(w) => match w.underlying_surface() {
				WindowSurface::Wayland(w) => {
					KeyboardTarget::modifiers(w.wl_surface(), seat, data, modifiers, serial);
				}
			},
			KeyboardFocusTarget::LayerSurface(l) => {
				KeyboardTarget::modifiers(l.wl_surface(), seat, data, modifiers, serial);
			}
			KeyboardFocusTarget::Popup(p) => {
				KeyboardTarget::modifiers(p.wl_surface(), seat, data, modifiers, serial);
			}
		}
	}
}

impl PointerTarget<State> for PointerFocusTarget {
	fn enter(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => PointerTarget::enter(w, seat, data, event),
			PointerFocusTarget::Window(w) => PointerTarget::enter(w, seat, data, event),
		}
	}

	fn motion(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => PointerTarget::motion(w, seat, data, event),
			PointerFocusTarget::Window(w) => PointerTarget::motion(w, seat, data, event),
		}
	}

	fn relative_motion(&self, seat: &Seat<State>, data: &mut State, event: &RelativeMotionEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => {
				PointerTarget::relative_motion(w, seat, data, event);
			}
			PointerFocusTarget::Window(w) => {
				PointerTarget::relative_motion(w, seat, data, event);
			}
		}
	}

	fn button(&self, seat: &Seat<State>, data: &mut State, event: &ButtonEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => PointerTarget::button(w, seat, data, event),
			PointerFocusTarget::Window(w) => PointerTarget::button(w, seat, data, event),
		}
	}

	fn axis(&self, seat: &Seat<State>, data: &mut State, frame: AxisFrame) {
		match self {
			PointerFocusTarget::WlSurface(w) => PointerTarget::axis(w, seat, data, frame),
			PointerFocusTarget::Window(w) => PointerTarget::axis(w, seat, data, frame),
		}
	}

	fn frame(&self, seat: &Seat<State>, data: &mut State) {
		match self {
			PointerFocusTarget::WlSurface(w) => PointerTarget::frame(w, seat, data),
			PointerFocusTarget::Window(w) => PointerTarget::frame(w, seat, data),
		}
	}

	fn gesture_swipe_begin(&self, seat: &Seat<State>, data: &mut State, event: &GestureSwipeBeginEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => {
				PointerTarget::gesture_swipe_begin(w, seat, data, event);
			}
			PointerFocusTarget::Window(w) => {
				PointerTarget::gesture_swipe_begin(w, seat, data, event);
			}
		}
	}

	fn gesture_swipe_update(&self, seat: &Seat<State>, data: &mut State, event: &GestureSwipeUpdateEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => {
				PointerTarget::gesture_swipe_update(w, seat, data, event);
			}
			PointerFocusTarget::Window(w) => {
				PointerTarget::gesture_swipe_update(w, seat, data, event);
			}
		}
	}

	fn gesture_swipe_end(&self, seat: &Seat<State>, data: &mut State, event: &GestureSwipeEndEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => {
				PointerTarget::gesture_swipe_end(w, seat, data, event);
			}
			PointerFocusTarget::Window(w) => {
				PointerTarget::gesture_swipe_end(w, seat, data, event);
			}
		}
	}

	fn gesture_pinch_begin(&self, seat: &Seat<State>, data: &mut State, event: &GesturePinchBeginEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => {
				PointerTarget::gesture_pinch_begin(w, seat, data, event);
			}
			PointerFocusTarget::Window(w) => {
				PointerTarget::gesture_pinch_begin(w, seat, data, event);
			}
		}
	}

	fn gesture_pinch_update(&self, seat: &Seat<State>, data: &mut State, event: &GesturePinchUpdateEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => {
				PointerTarget::gesture_pinch_update(w, seat, data, event);
			}
			PointerFocusTarget::Window(w) => {
				PointerTarget::gesture_pinch_update(w, seat, data, event);
			}
		}
	}

	fn gesture_pinch_end(&self, seat: &Seat<State>, data: &mut State, event: &GesturePinchEndEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => {
				PointerTarget::gesture_pinch_end(w, seat, data, event);
			}
			PointerFocusTarget::Window(w) => {
				PointerTarget::gesture_pinch_end(w, seat, data, event);
			}
		}
	}

	fn gesture_hold_begin(&self, seat: &Seat<State>, data: &mut State, event: &GestureHoldBeginEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => {
				PointerTarget::gesture_hold_begin(w, seat, data, event);
			}
			PointerFocusTarget::Window(w) => {
				PointerTarget::gesture_hold_begin(w, seat, data, event);
			}
		}
	}

	fn gesture_hold_end(&self, seat: &Seat<State>, data: &mut State, event: &GestureHoldEndEvent) {
		match self {
			PointerFocusTarget::WlSurface(w) => {
				PointerTarget::gesture_hold_end(w, seat, data, event);
			}
			PointerFocusTarget::Window(w) => {
				PointerTarget::gesture_hold_end(w, seat, data, event);
			}
		}
	}

	fn leave(&self, seat: &Seat<State>, data: &mut State, serial: smithay::utils::Serial, time: u32) {
		match self {
			PointerFocusTarget::WlSurface(w) => PointerTarget::leave(w, seat, data, serial, time),
			PointerFocusTarget::Window(w) => PointerTarget::leave(w, seat, data, serial, time),
		}
	}
}

impl WaylandFocus for KeyboardFocusTarget {
	fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
		match self {
			KeyboardFocusTarget::Window(window) => window.wl_surface(),
			KeyboardFocusTarget::LayerSurface(layer) => Some(Cow::Borrowed(layer.wl_surface())),
			KeyboardFocusTarget::Popup(popup) => Some(Cow::Borrowed(popup.wl_surface())),
		}
	}

	fn same_client_as(&self, object_id: &ObjectId) -> bool {
		match self {
			KeyboardFocusTarget::Window(window) => window.same_client_as(object_id),
			KeyboardFocusTarget::LayerSurface(layer) => layer.wl_surface().id().same_client_as(object_id),
			KeyboardFocusTarget::Popup(popup) => popup.wl_surface().id().same_client_as(object_id),
		}
	}
}

impl WaylandFocus for PointerFocusTarget {
	fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
		match self {
			PointerFocusTarget::WlSurface(w) => w.wl_surface(),
			PointerFocusTarget::Window(w) => w.wl_surface(),
		}
	}

	fn same_client_as(&self, object_id: &ObjectId) -> bool {
		match self {
			PointerFocusTarget::WlSurface(w) => w.same_client_as(object_id),
			PointerFocusTarget::Window(w) => w.same_client_as(object_id),
		}
	}
}

impl From<WlSurface> for PointerFocusTarget {
	fn from(wl_surface: WlSurface) -> Self {
		PointerFocusTarget::WlSurface(wl_surface)
	}
}

impl From<&WlSurface> for PointerFocusTarget {
	fn from(wl_surface: &WlSurface) -> Self {
		PointerFocusTarget::WlSurface(wl_surface.clone())
	}
}

impl From<PopupKind> for PointerFocusTarget {
	fn from(popup: PopupKind) -> Self {
		PointerFocusTarget::WlSurface(popup.wl_surface().clone())
	}
}

impl From<&MappedWindow> for PointerFocusTarget {
	fn from(window: &MappedWindow) -> Self {
		PointerFocusTarget::Window(window.clone())
	}
}

impl From<LayerSurface> for KeyboardFocusTarget {
	fn from(layer: LayerSurface) -> Self {
		KeyboardFocusTarget::LayerSurface(layer)
	}
}

impl From<&LayerSurface> for KeyboardFocusTarget {
	fn from(value: &LayerSurface) -> Self {
		KeyboardFocusTarget::LayerSurface(value.clone())
	}
}

impl From<PopupKind> for KeyboardFocusTarget {
	fn from(popup: PopupKind) -> Self {
		KeyboardFocusTarget::Popup(popup)
	}
}

impl From<MappedWindow> for KeyboardFocusTarget {
	fn from(window: MappedWindow) -> Self {
		KeyboardFocusTarget::Window(window)
	}
}
