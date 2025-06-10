use super::Relocate;
use crate::utils::{RectExt, output_size};
use mayland_config::outputs::OutputInfo;
use smithay::{
	output::Output,
	utils::{Logical, Point, Rectangle},
};
use std::cmp::Ordering;

type OutputLayout = (Output, Point<i32, Logical>);

#[derive(Debug)]
pub struct OutputSpace {
	pub active: Option<Output>,
	outputs: Vec<OutputLayout>,
}

impl OutputSpace {
	pub fn new() -> Self {
		OutputSpace {
			active: None,
			outputs: Vec::new(),
		}
	}

	#[must_use = "you have to reposition the cursor"]
	pub fn add_output(&mut self, config: &mayland_config::Outputs, output: &Output) -> Option<Relocate> {
		let active_position = self.active_output_position();

		// todo make this a little cleaner
		self.outputs.push((output.clone(), Point::new(0, 0)));
		self.reposition(config);

		let output_info = output.user_data().get::<OutputInfo>().unwrap();
		let output_config = config.get_output(output_info);

		if self.active.is_none() || output_config.is_some_and(|conf| conf.active) {
			self.active = Some(output.clone());

			let output_geometry = self.output_geometry(output).unwrap();
			Some(Relocate::Absolute(output_geometry.center()))
		} else if let Some(active_position) = active_position {
			let new_active_position = self.active_output_position().unwrap();
			if active_position != new_active_position {
				Some(Relocate::Relative(new_active_position - active_position))
			} else {
				None
			}
		} else {
			None
		}
	}

	#[must_use = "you have to reposition the cursor"]
	pub fn remove_output(&mut self, config: &mayland_config::Outputs, output: &Output) -> Option<Relocate> {
		let idx = self.outputs.iter().position(|(o, _)| o == output).unwrap();
		self.outputs.remove(idx);

		self.active.take_if(|active| active == output);
		let active_position = self.active_output_position();

		self.reposition(config);

		// the active output was not removed
		if let Some(active_position) = active_position {
			let new_active_position = self.active_output_position().unwrap();
			if active_position != new_active_position {
				Some(Relocate::Relative(new_active_position - active_position))
			} else {
				None
			}
		} else if let Some((new_active, position)) = self.outputs.first().cloned() {
			let size = output_size(&new_active);
			let output_geometry = Rectangle { loc: position, size };

			self.active = Some(new_active);
			Some(Relocate::Absolute(output_geometry.center()))
		} else {
			None
		}
	}

	/// reconfigure the outputs on the output space
	///
	/// has to be called e.g. when an output changes size or the output
	/// config changes
	#[must_use = "you have to reposition the cursor"]
	pub fn reconfigure(&mut self, config: &mayland_config::Outputs) -> Option<Relocate> {
		// if we have no active outputs, we do not have any outputs,
		// so we can short circuit and return immediately
		let active_position = self.active_output_position()?;

		self.reposition(config);

		// as we didn't short circuit earlier we must still have an active output
		let new_active_position = self.active_output_position().unwrap();
		if active_position != new_active_position {
			Some(Relocate::Relative(new_active_position - active_position))
		} else {
			None
		}
	}

	fn reposition(&mut self, config: &mayland_config::Outputs) {
		let outputs = self.outputs.drain(..).map(|(output, _)| {
			let output_info = output.user_data().get::<OutputInfo>().unwrap();
			let output_config = config.get_output(output_info);
			let output_position = output_config.and_then(|config| config.position);

			(output, output_position)
		});
		let mut outputs = outputs.collect::<Vec<_>>();

		// first sort by OutputInfo
		outputs.sort_by(|(out1, _), (out2, _)| {
			let info1 = out1.user_data().get::<OutputInfo>().unwrap();
			let info2 = out2.user_data().get::<OutputInfo>().unwrap();

			info1.cmp(info2)
		});

		// then put the outputs with an explicit position first,
		// sorting those by position as well.
		outputs.sort_by(|(_, pos1), (_, pos2)| match (pos1, pos2) {
			(Some(one), Some(two)) => one.cmp(two),
			(None, Some(_)) => Ordering::Greater,
			(Some(_), None) => Ordering::Less,
			(None, None) => Ordering::Equal,
		});

		for (output, position) in outputs {
			if let Some(position) = position {
				let point = Point::new(position[0], position[1]);
				let size = output_size(&output);
				let rect = Rectangle { loc: point, size };

				if let Some(overlaps) = self.overlaps(rect) {
					panic!(
						"new output {:?} at position {:?} overlaps with output {:?} at {:?}",
						output.name(),
						rect,
						overlaps.0.name(),
						overlaps
					);
				}

				output.change_current_state(None, None, None, Some(point));
				self.outputs.push((output, point));
			} else {
				let x = self
					.outputs
					.iter()
					.map(|(output, point)| {
						let size = output_size(output);
						point.x + size.w
					})
					.max()
					.unwrap_or(0);

				let point = Point::new(x, 0);
				output.change_current_state(None, None, None, Some(point));
				self.outputs.push((output, point));
			}
		}
	}

	fn overlaps(&self, rect: Rectangle<i32, Logical>) -> Option<(&Output, Point<i32, Logical>)> {
		self.outputs.iter().map(|(o, l)| (o, *l)).find(|(output, loc)| {
			let size = output_size(output);
			let geometry = Rectangle { loc: *loc, size };

			geometry.overlaps(rect)
		})
	}

	pub fn active_output_position(&self) -> Option<Point<i32, Logical>> {
		let active = self.active.as_ref()?;
		let position = self.output_position(active).unwrap();
		Some(position)
	}

	pub fn refresh(&self) {
		for (output, _) in &self.outputs {
			output.cleanup();
		}
	}

	pub fn output_geometry(&self, output: &Output) -> Option<Rectangle<i32, Logical>> {
		let (output, location) = self.outputs.iter().find(|(o, _)| o == output)?;

		let size = output_size(output);
		let geometry = Rectangle { loc: *location, size };
		Some(geometry)
	}

	pub fn output_position(&self, output: &Output) -> Option<Point<i32, Logical>> {
		self.outputs.iter().find(|(o, _)| o == output).map(|(_, p)| *p)
	}

	pub fn output_under(&self, point: Point<f64, Logical>) -> Option<&Output> {
		self.outputs.iter().find_map(|(output, location)| {
			if point.x < location.x as f64 || point.y < location.y as f64 {
				None
			} else {
				let size = output_size(output);
				let geometry = Rectangle { loc: *location, size };

				geometry.to_f64().contains(point).then_some(output)
			}
		})
	}

	pub fn outputs_geometry(
		&self,
	) -> impl DoubleEndedIterator<Item = (&Output, Rectangle<i32, Logical>)> + ExactSizeIterator {
		self.outputs.iter().map(|(output, location)| {
			let size = output_size(output);
			(output, Rectangle::new(*location, size))
		})
	}

	pub fn outputs(&self) -> impl DoubleEndedIterator<Item = &Output> + ExactSizeIterator {
		self.outputs.iter().map(|(output, _)| output)
	}
}
