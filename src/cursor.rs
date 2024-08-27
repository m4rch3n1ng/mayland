use smithay::{
	backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
	input::pointer::{CursorIcon, CursorImageStatus, CursorImageSurfaceData},
	reexports::wayland_server::protocol::wl_surface::WlSurface,
	utils::{Logical, Physical, Point, Transform},
	wayland::compositor::with_states,
};
use std::{collections::HashMap, fmt::Debug, io::Read, time::Duration};
use xcursor::{parser::Image, CursorTheme};

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

	theme: CursorTheme,
	size: u32,

	cache: HashMap<CursorIcon, Option<XCursor>>,
}

impl Cursor {
	pub fn new() -> Self {
		let (theme, size) = load_cursor_theme();

		Cursor {
			status: CursorImageStatus::default_named(),
			icon: None,

			theme,
			size,

			cache: HashMap::new(),
		}
	}

	fn get_named_cursor(&mut self, icon: CursorIcon) -> Option<&mut XCursor> {
		self.cache
			.entry(icon)
			.or_insert_with(|| {
				// todo scale
				let size = self.size;

				if let Some(xcursor) = XCursor::load(&self.theme, icon.name(), size) {
					Some(xcursor)
				} else if icon == CursorIcon::Default {
					let xcursor = XCursor::fallback_cursor();
					Some(xcursor)
				} else {
					None
				}
			})
			.as_mut()
	}

	fn get_default_cursor(&mut self) -> &mut XCursor {
		self.get_named_cursor(CursorIcon::Default)
			.expect("CursorIcon::Default should always be populated")
	}

	// todo scale
	pub fn get_render_cursor(&mut self, _scale: i32) -> RenderCursor {
		if let Some(xcursor) = self.icon.and_then(|icon| self.get_named_cursor(icon)) {
			let xcursor = xcursor as *mut XCursor;

			// SAFETY: see safety comment further down below.
			//
			// this code snippet also compiles normally
			// with `-Zpolonius` and miri doesn't complain either.
			let xcursor = unsafe { &mut *xcursor };
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
				let xcursor = match self.get_named_cursor(icon) {
					Some(xcursor) => xcursor as *mut XCursor,
					None => self.get_default_cursor() as *mut XCursor,
				};

				// SAFETY: this shouldn't break the rust aliasing rules,
				// as there is only one (1) mutable reference at a time,
				// and the 'a reflects the actual lifetime of the data,
				// which makes it a compiler error to borrow immutably twice.
				//
				// the pointer is also properly aligned, non-null and
				// dereferenceable.
				//
				// i wouldn't normally use unsafe for something like this,
				// but with `-Zpolonius` the code compiles even without
				// any raw pointer fuckery.
				// miri also seems happy with this code.
				let xcursor = unsafe { &mut *xcursor };
				RenderCursor::Named(xcursor)
			}
		}
	}
}

fn load_cursor_theme() -> (CursorTheme, u32) {
	let name = std::env::var("XCURSOR_THEME").unwrap_or_else(|_| "default".to_owned());
	let theme = CursorTheme::load(&name);

	let size = std::env::var("XCURSOR_SIZE")
		.ok()
		.and_then(|s| s.parse().ok())
		.unwrap_or(24);

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
				&self.image.pixels_rgba,
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
	fn load(theme: &CursorTheme, name: &str, size: u32) -> Option<Self> {
		let icon_path = theme.load_icon(name)?;
		let mut cursor_file = std::fs::File::open(icon_path).ok()?;
		let mut cursor_data = Vec::new();
		cursor_file.read_to_end(&mut cursor_data).ok()?;

		let mut images = xcursor::parser::parse_xcursor(&cursor_data)?;

		let (width, height) = images
			.iter()
			.min_by_key(|image| u32::abs_diff(image.size, size))
			.map(|image| (image.width, image.height))
			.unwrap();

		images.retain(|image| image.width == width && image.height == height);

		let animation_duration = images.iter().fold(0, |acc, image| acc + image.delay);
		let frames = images.into_iter().map(Frame::new).collect();

		Some(XCursor {
			frames,
			animation_duration,
		})
	}

	fn fallback_cursor() -> Self {
		let icon = Image {
			size: 32,
			width: 64,
			height: 64,
			xhot: 1,
			yhot: 1,
			delay: 0,
			pixels_rgba: Vec::from(FALLBACK_CURSOR_DATA),
			pixels_argb: vec![],
		};
		let frame = Frame::new(icon);

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
