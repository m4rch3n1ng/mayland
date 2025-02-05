use serde::{
	de::{VariantAccess, Visitor},
	Deserialize,
};
use smithay::input::keyboard::XkbConfig;
use smithay::reexports::input::{AccelProfile, ClickMethod, ScrollMethod, TapButtonMap};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Input {
	pub keyboard: Keyboard,
	touchpad: Touchpad,
	mouse: Mouse,

	devices: Vec<Device>,
}

#[derive(Debug, PartialEq, Eq)]
enum Device {
	Touchpad(String, Touchpad),
	Mouse(String, Mouse),
}

impl Input {
	pub fn touchpad(&self, name: &str) -> &Touchpad {
		self.devices
			.iter()
			.filter_map(|device| match device {
				Device::Touchpad(name, touchpad) => Some((name, touchpad)),
				_ => None,
			})
			.find(|(n, _)| n.eq_ignore_ascii_case(name))
			.map(|(_, touchpad)| touchpad)
			.unwrap_or(&self.touchpad)
	}

	pub fn mouse(&self, name: &str) -> &Mouse {
		self.devices
			.iter()
			.filter_map(|device| match device {
				Device::Mouse(name, mouse) => Some((name, mouse)),
				_ => None,
			})
			.find(|(n, _)| n.eq_ignore_ascii_case(name))
			.map(|(_, mouse)| mouse)
			.unwrap_or(&self.mouse)
	}
}

impl<'de> Deserialize<'de> for Input {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		enum DeviceField {
			Touchpad(String),
			Mouse(String),
		}

		enum Field {
			Keyboard,
			Touchpad,
			Mouse,

			Device(DeviceField),

			Ignore,
		}

		struct FieldVis;

		impl<'de> Visitor<'de> for FieldVis {
			type Value = Field;

			fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				f.write_str("an input key")
			}

			fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
				match v {
					"keyboard" => Ok(Field::Keyboard),
					"touchpad" => Ok(Field::Touchpad),
					"mouse" => Ok(Field::Mouse),
					_ => Ok(Field::Ignore),
				}
			}

			fn visit_enum<A: serde::de::EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
				let (tag, val) = data.variant::<String>()?;

				match &*tag {
					"keyboard" => {
						let _ = val.newtype_variant::<String>();
						Err(serde::de::Error::custom(
							"per-device configuration is not yet implement for keyboards",
						))
					}
					"touchpad" => {
						let name = val.newtype_variant::<String>()?;
						let name = DeviceField::Touchpad(name);
						Ok(Field::Device(name))
					}
					"mouse" => {
						let name = val.newtype_variant::<String>()?;
						let name = DeviceField::Mouse(name);
						Ok(Field::Device(name))
					}
					_ => {
						let _ = val.newtype_variant::<serde::de::IgnoredAny>();
						Ok(Field::Ignore)
					}
				}
			}
		}

		impl<'de> Deserialize<'de> for Field {
			fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
				deserializer.deserialize_any(FieldVis)
			}
		}

		struct InputVisitor;

		impl<'de> Visitor<'de> for InputVisitor {
			type Value = Input;

			fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				f.write_str("the input config")
			}

			fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
				let mut keyboard = None;
				let mut touchpad = None;
				let mut mouse = None;

				enum TmpDevice {
					Touchpad(String, per_device::Touchpad),
					Mouse(String, per_device::Mouse),
				}

				let mut devices = Vec::new();

				while let Some(key) = map.next_key::<Field>()? {
					match key {
						Field::Keyboard => {
							assert!(keyboard.is_none());
							keyboard = Some(map.next_value::<Keyboard>()?);
						}
						Field::Touchpad => {
							assert!(touchpad.is_none());
							touchpad = Some(map.next_value::<Touchpad>()?);
						}
						Field::Mouse => {
							assert!(mouse.is_none());
							mouse = Some(map.next_value::<Mouse>()?);
						}

						Field::Device(device) => match device {
							DeviceField::Touchpad(dev) => {
								let touchpad = map.next_value::<per_device::Touchpad>()?;
								let device = TmpDevice::Touchpad(dev, touchpad);
								devices.push(device);
							}
							DeviceField::Mouse(dev) => {
								let mouse = map.next_value::<per_device::Mouse>()?;
								let device = TmpDevice::Mouse(dev, mouse);
								devices.push(device);
							}
						},

						Field::Ignore => {
							let _ = map.next_value::<serde::de::IgnoredAny>();
						}
					}
				}

				let keyboard = keyboard.unwrap_or_default();
				let touchpad = touchpad.unwrap_or_default();
				let mouse = mouse.unwrap_or_default();

				let devices = devices
					.into_iter()
					.map(|device| match device {
						TmpDevice::Touchpad(dev, value) => Device::Touchpad(dev, value.merge(&touchpad)),
						TmpDevice::Mouse(dev, value) => Device::Mouse(dev, value.merge(&mouse)),
					})
					.collect();

				let input = Input {
					keyboard,
					touchpad,
					mouse,

					devices,
				};
				Ok(input)
			}
		}

		deserializer.deserialize_map(InputVisitor)
	}
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Keyboard {
	#[serde(deserialize_with = "deserialize_path")]
	pub xkb_file: Option<String>,

	pub xkb_rules: String,
	pub xkb_layout: String,
	pub xkb_model: String,
	pub xkb_variant: String,
	pub xkb_options: Option<String>,

	pub repeat_delay: i32,
	pub repeat_rate: i32,
}

impl Keyboard {
	pub fn xkb_config(&self) -> XkbConfig<'_> {
		XkbConfig {
			rules: &self.xkb_rules,
			model: &self.xkb_model,
			layout: &self.xkb_layout,
			variant: &self.xkb_variant,
			options: self.xkb_options.clone(),
		}
	}
}

impl Default for Keyboard {
	fn default() -> Self {
		Keyboard {
			xkb_file: None,

			xkb_rules: String::new(),
			xkb_model: String::new(),
			xkb_layout: String::new(),
			xkb_variant: String::new(),
			xkb_options: None,

			repeat_delay: 600,
			repeat_rate: 25,
		}
	}
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(default)]
pub struct Touchpad {
	pub tap: bool,
	pub tap_and_drag: bool,
	pub tap_drag_lock: bool,

	pub dwt: bool,
	pub dwtp: bool,

	pub natural_scroll: bool,
	#[serde(with = "scroll_method")]
	pub scroll_method: Option<ScrollMethod>,

	#[serde(with = "click_method")]
	pub click_method: Option<ClickMethod>,

	pub middle_emulation: bool,
	#[serde(with = "tap_button_map")]
	pub tap_button_map: Option<TapButtonMap>,
	pub left_handed: bool,

	pub accel_speed: f64,
	#[serde(with = "accel_profile")]
	pub accel_profile: Option<AccelProfile>,
}

/// all values are parsed with [`mayfig`],
/// which does not support nan floats
impl Eq for Touchpad {}

impl Default for Touchpad {
	fn default() -> Self {
		Touchpad {
			tap: true,
			tap_and_drag: false,
			tap_drag_lock: false,

			dwt: true,
			dwtp: true,

			natural_scroll: true,
			scroll_method: None,

			click_method: None,

			middle_emulation: true,
			tap_button_map: None,
			left_handed: false,

			accel_speed: 0.0,
			accel_profile: None,
		}
	}
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(default)]
pub struct Mouse {
	pub natural_scroll: bool,

	pub middle_emulation: bool,
	pub left_handed: bool,

	pub accel_speed: f64,
	#[serde(with = "accel_profile")]
	pub accel_profile: Option<AccelProfile>,
}

/// all values are parsed with [`mayfig`],
/// which does not support nan floats
impl Eq for Mouse {}

impl Default for Mouse {
	fn default() -> Self {
		Mouse {
			natural_scroll: false,

			middle_emulation: false,
			left_handed: false,

			accel_speed: 0.0,
			accel_profile: None,
		}
	}
}

fn deserialize_path<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
	let option = Option::<String>::deserialize(deserializer)?;
	let Some(mut path) = option else { return Ok(None) };

	let path = if let Some(rest) = path.strip_prefix("~") {
		let home = dirs::home_dir().unwrap();
		let home = home.into_os_string().into_string().unwrap();
		if rest.is_empty() {
			home
		} else {
			path.replace_range(0..=0, &home);
			path
		}
	} else {
		path
	};

	Ok(Some(path))
}

mod per_device {
	use serde::Deserialize;
	use smithay::reexports::input::{AccelProfile, ClickMethod, ScrollMethod, TapButtonMap};

	#[derive(Default, Deserialize)]
	#[serde(default)]
	pub struct Mouse {
		pub natural_scroll: Option<bool>,

		pub middle_emulation: Option<bool>,
		pub left_handed: Option<bool>,

		pub accel_speed: Option<f64>,
		#[serde(with = "super::accel_profile")]
		pub accel_profile: Option<AccelProfile>,
	}

	impl Mouse {
		pub fn merge(self, other: &super::Mouse) -> super::Mouse {
			super::Mouse {
				natural_scroll: self.natural_scroll.unwrap_or(other.natural_scroll),

				middle_emulation: self.middle_emulation.unwrap_or(other.middle_emulation),
				left_handed: self.left_handed.unwrap_or(other.left_handed),

				accel_speed: self.accel_speed.unwrap_or(other.accel_speed),
				accel_profile: self.accel_profile.or(other.accel_profile),
			}
		}
	}

	#[derive(Default, Deserialize)]
	#[serde(default)]
	pub struct Touchpad {
		pub tap: Option<bool>,
		pub tap_and_drag: Option<bool>,
		pub tap_drag_lock: Option<bool>,

		pub dwt: Option<bool>,
		pub dwtp: Option<bool>,

		pub natural_scroll: Option<bool>,
		#[serde(with = "super::scroll_method")]
		pub scroll_method: Option<ScrollMethod>,

		#[serde(with = "super::click_method")]
		pub click_method: Option<ClickMethod>,

		pub middle_emulation: Option<bool>,
		#[serde(with = "super::tap_button_map")]
		pub tap_button_map: Option<TapButtonMap>,
		pub left_handed: Option<bool>,

		pub accel_speed: Option<f64>,
		#[serde(with = "super::accel_profile")]
		pub accel_profile: Option<AccelProfile>,
	}

	impl Touchpad {
		pub fn merge(self, other: &super::Touchpad) -> super::Touchpad {
			super::Touchpad {
				tap: self.tap.unwrap_or(other.tap),
				tap_and_drag: self.tap_and_drag.unwrap_or(other.tap_and_drag),
				tap_drag_lock: self.tap_drag_lock.unwrap_or(other.tap_drag_lock),

				dwt: self.dwt.unwrap_or(other.dwt),
				dwtp: self.dwtp.unwrap_or(other.dwtp),

				natural_scroll: self.natural_scroll.unwrap_or(other.natural_scroll),
				scroll_method: self.scroll_method.or(other.scroll_method),

				click_method: self.click_method.or(other.click_method),

				middle_emulation: self.middle_emulation.unwrap_or(other.middle_emulation),
				tap_button_map: self.tap_button_map.or(other.tap_button_map),
				left_handed: self.left_handed.unwrap_or(other.left_handed),

				accel_speed: self.accel_speed.unwrap_or(other.accel_speed),
				accel_profile: self.accel_profile.or(other.accel_profile),
			}
		}
	}
}

mod click_method {
	use serde::{Deserialize, Deserializer};
	use smithay::reexports::input as libinput;

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "snake_case")]
	enum ClickMethod {
		ButtonAreas,
		Clickfinger,
	}

	impl From<ClickMethod> for libinput::ClickMethod {
		fn from(value: ClickMethod) -> Self {
			match value {
				ClickMethod::ButtonAreas => libinput::ClickMethod::ButtonAreas,
				ClickMethod::Clickfinger => libinput::ClickMethod::Clickfinger,
			}
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<libinput::ClickMethod>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let option = Option::<ClickMethod>::deserialize(deserializer)?;
		let option = option.map(libinput::ClickMethod::from);
		Ok(option)
	}
}

mod tap_button_map {
	use serde::{Deserialize, Deserializer};
	use smithay::reexports::input as libinput;

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "snake_case")]
	enum TapButtonMap {
		LeftRightMiddle,
		LeftMiddleRight,
	}

	impl From<TapButtonMap> for libinput::TapButtonMap {
		fn from(value: TapButtonMap) -> Self {
			match value {
				TapButtonMap::LeftRightMiddle => libinput::TapButtonMap::LeftRightMiddle,
				TapButtonMap::LeftMiddleRight => libinput::TapButtonMap::LeftMiddleRight,
			}
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<libinput::TapButtonMap>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let option = Option::<TapButtonMap>::deserialize(deserializer)?;
		let option = option.map(libinput::TapButtonMap::from);
		Ok(option)
	}
}

mod scroll_method {
	use serde::{Deserialize, Deserializer};
	use smithay::reexports::input as libinput;

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "snake_case")]
	enum ScrollMethod {
		NoScroll,
		TwoFinger,
		Edge,
		OnButtonDown,
	}

	impl From<ScrollMethod> for libinput::ScrollMethod {
		fn from(value: ScrollMethod) -> Self {
			match value {
				ScrollMethod::NoScroll => libinput::ScrollMethod::NoScroll,
				ScrollMethod::TwoFinger => libinput::ScrollMethod::TwoFinger,
				ScrollMethod::Edge => libinput::ScrollMethod::Edge,
				ScrollMethod::OnButtonDown => libinput::ScrollMethod::OnButtonDown,
			}
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<libinput::ScrollMethod>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let option = Option::<ScrollMethod>::deserialize(deserializer)?;
		let option = option.map(libinput::ScrollMethod::from);
		Ok(option)
	}
}

mod accel_profile {
	use serde::{Deserialize, Deserializer};
	use smithay::reexports::input as libinput;

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "snake_case")]
	enum AccelProfile {
		Adaptive,
		Flat,
	}

	impl From<AccelProfile> for libinput::AccelProfile {
		fn from(value: AccelProfile) -> Self {
			match value {
				AccelProfile::Adaptive => libinput::AccelProfile::Adaptive,
				AccelProfile::Flat => libinput::AccelProfile::Flat,
			}
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<libinput::AccelProfile>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let option = Option::<AccelProfile>::deserialize(deserializer)?;
		let option = option.map(libinput::AccelProfile::from);
		Ok(option)
	}
}
