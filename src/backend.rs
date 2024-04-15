pub use self::winit::Winit;
use crate::state::Mayland;
use smithay::backend::allocator::dmabuf::Dmabuf;

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

	pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
		match self {
			Backend::Winit(winit) => winit.import_dmabuf(dmabuf),
		}
	}

	pub fn winit(&mut self) -> &mut Winit {
		match self {
			Backend::Winit(winit) => winit,
		}
	}
}
