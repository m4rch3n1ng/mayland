use self::{udev::Udev, winit::Winit};
use crate::{render::MaylandRenderElements, state::Mayland};
use smithay::{
	backend::{allocator::dmabuf::Dmabuf, renderer::glow::GlowRenderer},
	output::Output,
};

pub mod udev;
pub mod winit;

#[derive(Debug)]
pub enum Backend {
	Udev(Udev),
	Winit(Winit),
}

impl Backend {
	pub fn render(&mut self, mayland: &mut Mayland, output: &Output, elements: &[MaylandRenderElements]) {
		match self {
			Backend::Udev(udev) => udev.render(mayland, output, elements),
			Backend::Winit(winit) => winit.render(mayland, output, elements),
		}
	}

	pub fn renderer(&mut self) -> &mut GlowRenderer {
		match self {
			Backend::Udev(udev) => udev.renderer(),
			Backend::Winit(winit) => winit.renderer(),
		}
	}

	pub fn switch_vt(&mut self, vt: i32) {
		match self {
			Backend::Udev(udev) => udev.switch_vt(vt),
			Backend::Winit(_) => (),
		}
	}

	pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
		match self {
			Backend::Udev(udev) => udev.import_dmabuf(dmabuf),
			Backend::Winit(winit) => winit.import_dmabuf(dmabuf),
		}
	}

	pub fn comm_outputs(&self, mayland: &Mayland) -> Vec<mayland_comm::Output> {
		match self {
			Backend::Udev(udev) => udev.comm_outputs(mayland),
			Backend::Winit(winit) => winit.comm_outputs(),
		}
	}

	pub fn reload_output_config(&mut self, mayland: &mut Mayland, config: &mayland_config::Outputs) {
		match self {
			Backend::Udev(udev) => udev.reload_output_config(mayland, config),
			Backend::Winit(winit) => winit.reload_output_config(mayland, config),
		}
	}

	pub fn winit(&mut self) -> &mut Winit {
		match self {
			Backend::Udev(_udev) => unreachable!("should only be called in winit context"),
			Backend::Winit(winit) => winit,
		}
	}

	pub fn udev(&mut self) -> &mut Udev {
		match self {
			Backend::Udev(udev) => udev,
			Backend::Winit(_winit) => unreachable!("should only be called in udev context"),
		}
	}
}
