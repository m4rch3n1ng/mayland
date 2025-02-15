use smithay::utils::{Logical, Point};

pub mod floating;
pub mod outputs;
pub mod tiling;
pub mod workspace;

#[derive(Debug, Clone, Copy)]
pub enum Relocate {
	Absolute(Point<i32, Logical>),
	Relative(Point<i32, Logical>),
}
