use smithay::reexports::input::{self as libinput, DeviceCapability};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputDeviceType {
	Keyboard,
	Pointer,
	Touch,
	TabletTool,
	TabletPad,
	Switch,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputDevice {
	pub handle: libinput::Device,
	pub r#type: InputDeviceType,
}

impl InputDevice {
	pub fn split(dev: &libinput::Device) -> Vec<Self> {
		const TYPES: [(DeviceCapability, InputDeviceType); 6] = [
			(DeviceCapability::Pointer, InputDeviceType::Pointer),
			(DeviceCapability::Keyboard, InputDeviceType::Keyboard),
			(DeviceCapability::Touch, InputDeviceType::Touch),
			(DeviceCapability::TabletTool, InputDeviceType::TabletTool),
			(DeviceCapability::TabletPad, InputDeviceType::TabletPad),
			(DeviceCapability::Switch, InputDeviceType::Switch),
		];

		let mut devices = vec![];

		for (cap, r#type) in TYPES {
			if dev.has_capability(cap) {
				let device = InputDevice {
					handle: dev.clone(),
					r#type,
				};
				devices.push(device);
			}
		}

		devices
	}

	pub fn is_touchpad(&self) -> bool {
		self.r#type == InputDeviceType::Pointer && self.handle.config_tap_finger_count() > 0
	}
}
