use serde::{de::Visitor, Deserialize};
use std::{cmp::Ordering, collections::HashMap};

#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Outputs(HashMap<String, Output>);

#[derive(Debug, Clone)]
pub struct OutputInfo {
	pub connector: String,
	pub make: String,
	pub model: String,
	pub serial: Option<String>,
}

impl Eq for OutputInfo {}

impl PartialEq for OutputInfo {
	fn eq(&self, other: &Self) -> bool {
		self.connector == other.connector
	}
}

impl Ord for OutputInfo {
	/// tries to provide a relatively consistent ordering for
	/// outputs, so that i can always map them in that order
	///
	/// puts internal monitors ("eDP-1") first, and then sorts by
	/// the connector name
	fn cmp(&self, other: &Self) -> Ordering {
		match (&*self.connector, &*other.connector) {
			("eDP-1", "eDP-1") => Ordering::Equal,
			("eDP-1", _) => Ordering::Less,
			(_, "eDP-1") => Ordering::Greater,
			(conn1, conn2) => conn1.cmp(conn2),
		}
	}
}

impl PartialOrd for OutputInfo {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Outputs {
	pub fn get_output(&self, info: &OutputInfo) -> Option<&Output> {
		self.0.get(&info.connector)
	}
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Output {
	pub mode: Option<Mode>,
	pub active: bool,
	pub position: Option<[i32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
	pub width: u16,
	pub height: u16,
	pub refresh: Option<i32>,
}

impl<'de> Deserialize<'de> for Mode {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		deserializer.deserialize_str(ModeVis)
	}
}

struct ModeVis;

impl Visitor<'_> for ModeVis {
	type Value = Mode;

	fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("a valid mode")
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		let (size, refresh) = v.split_once('@').map(|(s, r)| (s, Some(r))).unwrap_or((v, None));

		let Some((width, height)) = size.split_once('x') else {
			return Err(serde::de::Error::custom(format_args!("invalid size {:?}", size)));
		};

		let width = width
			.parse::<u16>()
			.map_err(|err| serde::de::Error::custom(format_args!("invalid width {:?} ({})", width, err)))?;
		let height = height
			.parse::<u16>()
			.map_err(|err| serde::de::Error::custom(format_args!("invalid height {:?} ({})", height, err)))?;

		let refresh = if let Some(refresh) = refresh {
			let refresh = refresh.parse::<f32>().map_err(|err| {
				serde::de::Error::custom(format_args!("invalid refresh {:?} ({})", refresh, err))
			})?;
			let refresh = (refresh * 1000.).round() as i32;
			Some(refresh)
		} else {
			None
		};

		Ok(Mode {
			width,
			height,
			refresh,
		})
	}
}
