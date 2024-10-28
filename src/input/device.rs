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
		// this is how niri checks for touchpads
		self.r#type == InputDeviceType::Pointer && self.handle.config_tap_finger_count() > 0
	}

	fn is_trackball(&self) -> bool {
		// this is how mutter checks for trackballs

		// SAFETY: https://github.com/Smithay/input.rs/issues/76
		if let Some(udev_device) = unsafe { self.handle.udev_device() } {
			self.r#type == InputDeviceType::Pointer
				&& udev_device.property_value("ID_INPUT_TRACKBALL").is_some()
		} else {
			false
		}
	}

	fn is_trackpoint(&self) -> bool {
		// this is how mutter checks for trackpoints

		// SAFETY: https://github.com/Smithay/input.rs/issues/76
		if let Some(udev_device) = unsafe { self.handle.udev_device() } {
			self.r#type == InputDeviceType::Pointer
				&& udev_device.property_value("ID_INPUT_POINTINGSTICK").is_some()
		} else {
			false
		}
	}

	pub fn is_mouse(&self) -> bool {
		self.r#type == InputDeviceType::Pointer
			&& !self.is_touchpad()
			&& !self.is_trackball()
			&& !self.is_trackpoint()
	}
}
