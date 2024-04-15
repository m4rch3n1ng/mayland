pub use self::winit::Winit;
use crate::state::Mayland;

pub mod winit;

#[derive(Debug)]
pub enum Backend {
	Winit(Winit),
}

impl Backend {
	pub fn render(&mut self, mayland: &mut Mayland) {
		match self {
			Backend::Winit(winit) => winit.render(mayland),
		}
	}

	pub fn winit(&mut self) -> &mut Winit {
		match self {
			Backend::Winit(winit) => winit,
		}
	}
}
