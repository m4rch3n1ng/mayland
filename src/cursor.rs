use kcursor::{CursorTheme, Image};
use smithay::{
	backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
	input::pointer::{CursorIcon, CursorImageStatus, CursorImageSurfaceData},
	reexports::wayland_server::protocol::wl_surface::WlSurface,
	utils::{Logical, Physical, Point, Transform},
	wayland::compositor::with_states,
};
use std::{collections::HashMap, fmt::Debug, time::Duration};

const FALLBACK_CURSOR_DATA: &[u8] = include_bytes!("../resources/cursor.rgba");

pub enum RenderCursor<'a> {
	Hidden,
	Surface {
		hotspot: Point<i32, Logical>,
		surface: WlSurface,
	},
	Named(&'a mut XCursor),
}

#[derive(Debug)]
pub struct Cursor {
	pub status: CursorImageStatus,
	/// manually set the [`CursorIcon`] to override what the applications provide
	pub icon: Option<CursorIcon>,

	theme: Option<CursorTheme>,
	size: u32,

	cache: HashMap<CursorIcon, XCursor>,
}

impl Cursor {
	pub fn new(config: &mayland_config::Cursor, environment: &mut HashMap<String, String>) -> Self {
		let (theme, size) = load_cursor_theme(config, environment);

		Cursor {
			status: CursorImageStatus::default_named(),
			icon: None,

			theme,
			size,

			cache: HashMap::new(),
		}
	}

	pub fn reconfigure(
		&mut self,
		config: &mayland_config::Cursor,
		environment: &mut HashMap<String, String>,
	) {
		let (theme, size) = load_cursor_theme(config, environment);

		self.theme = theme;
		self.size = size;

		self.cache.clear();
	}

	fn get_named_cursor(&mut self, icon: CursorIcon) -> &mut XCursor {
		self.cache.entry(icon).or_insert_with(|| {
			// todo scale
			let size = self.size;

			let Some(theme) = &self.theme else {
				return XCursor::fallback_cursor();
			};

			XCursor::load(theme, icon, size)
				.or_else(|| XCursor::load(theme, CursorIcon::Default, size))
				.unwrap_or_else(XCursor::fallback_cursor)
		})
	}

	// todo scale
	pub fn get_render_cursor(&mut self, _scale: i32) -> RenderCursor<'_> {
		if let Some(icon) = self.icon {
			let xcursor = self.get_named_cursor(icon);
			return RenderCursor::Named(xcursor);
		}

		match self.status.clone() {
			CursorImageStatus::Hidden => RenderCursor::Hidden,
			CursorImageStatus::Surface(surface) => {
				let hotspot = with_states(&surface, |states| {
					states
						.data_map
						.get::<CursorImageSurfaceData>()
						.unwrap()
						.lock()
						.unwrap()
						.hotspot
				});

				RenderCursor::Surface { hotspot, surface }
			}
			CursorImageStatus::Named(icon) => {
				let xcursor = self.get_named_cursor(icon);
				RenderCursor::Named(xcursor)
			}
		}
	}
}

fn load_cursor_theme(
	config: &mayland_config::Cursor,
	environment: &mut HashMap<String, String>,
) -> (Option<CursorTheme>, u32) {
	let theme_name = (config.xcursor_theme.clone())
		.or_else(|| std::env::var("XCURSOR_THEME").ok())
		.unwrap_or_else(|| "default".to_owned());
	let theme = CursorTheme::load(&theme_name).or_else(|| CursorTheme::load("default"));

	let size = (config.xcursor_size)
		.or_else(|| std::env::var("XCURSOR_SIZE").ok().and_then(|s| s.parse().ok()))
		.unwrap_or(24);

	environment.insert("XCURSOR_THEME".to_owned(), theme_name);
	environment.insert("XCURSOR_SIZE".to_owned(), size.to_string());

	(theme, size)
}

#[derive(Debug)]
pub struct Frame {
	image: Image,
	buffer: Option<MemoryRenderBuffer>,
}

impl Frame {
	pub fn new(image: Image) -> Self {
		Frame { image, buffer: None }
	}

	pub fn hotspot(&self) -> Point<i32, Physical> {
		Point::from((self.image.xhot as i32, self.image.yhot as i32))
	}

	pub fn buffer(&mut self) -> &MemoryRenderBuffer {
		self.buffer.get_or_insert_with(|| {
			MemoryRenderBuffer::from_slice(
				&self.image.pixels,
				Fourcc::Argb8888,
				(self.image.width as i32, self.image.height as i32),
				1,
				Transform::Normal,
				None,
			)
		})
	}
}

#[derive(Debug)]
pub struct XCursor {
	frames: Vec<Frame>,
	animation_duration: u32,
}

impl XCursor {
	fn load(theme: &CursorTheme, icon: CursorIcon, size: u32) -> Option<Self> {
		let cursor = theme.icon(icon.name())?;
		let images = cursor.frames(size)?;

		let animation_duration = images.iter().map(|img| img.delay).sum::<u32>();
		let frames = images.into_iter().map(Frame::new).collect();

		Some(XCursor {
			frames,
			animation_duration,
		})
	}

	fn fallback_cursor() -> Self {
		let image = Image {
			size: 32,
			width: 64,
			height: 64,
			xhot: 1,
			yhot: 1,
			delay: 0,
			pixels: Vec::from(FALLBACK_CURSOR_DATA),
		};
		let frame = Frame::new(image);

		XCursor {
			frames: vec![frame],
			animation_duration: 0,
		}
	}

	pub fn frame(&mut self, duration: Duration) -> &mut Frame {
		if self.animation_duration == 0 {
			return &mut self.frames[0];
		}

		let mut millis = duration.as_millis() as u32;
		millis %= self.animation_duration;

		for frame in &mut self.frames {
			if millis < frame.image.delay {
				return frame;
			}
			millis -= frame.image.delay;
		}

		unreachable!();
	}
}
