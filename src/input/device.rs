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
	pub fn new(dev: &libinput::Device) -> Vec<Self> {
		let mut devices = vec![];

		if dev.has_capability(DeviceCapability::Keyboard) {
			let device = InputDevice {
				handle: dev.clone(),
				r#type: InputDeviceType::Keyboard,
			};
			devices.push(device);
		}

		if dev.has_capability(DeviceCapability::Pointer) {
			let device = InputDevice {
				handle: dev.clone(),
				r#type: InputDeviceType::Pointer,
			};
			devices.push(device);
		}

		if dev.has_capability(DeviceCapability::Touch) {
			let device = InputDevice {
				handle: dev.clone(),
				r#type: InputDeviceType::Touch,
			};
			devices.push(device);
		}

		if dev.has_capability(DeviceCapability::TabletTool) {
			let device = InputDevice {
				handle: dev.clone(),
				r#type: InputDeviceType::TabletTool,
			};
			devices.push(device);
		}

		if dev.has_capability(DeviceCapability::TabletPad) {
			let device = InputDevice {
				handle: dev.clone(),
				r#type: InputDeviceType::TabletPad,
			};
			devices.push(device);
		}

		if dev.has_capability(DeviceCapability::Switch) {
			let device = InputDevice {
				handle: dev.clone(),
				r#type: InputDeviceType::Switch,
			};
			devices.push(device);
		}

		devices
	}

	pub fn is_touchpad(&self) -> bool {
		self.r#type == InputDeviceType::Pointer && self.handle.config_tap_finger_count() > 0
	}
}
