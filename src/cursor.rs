use cursor_icon::CursorIcon;
use smithay::{
	backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
	input::pointer::CursorImageStatus,
	utils::{Physical, Point, Transform},
};
use std::{collections::HashMap, fmt::Debug, io::Read};
use xcursor::{parser::Image, CursorTheme};

const FALLBACK_CURSOR_DATA: &[u8] = include_bytes!("../resources/cursor.rgba");

#[derive(Debug)]
pub struct Cursor {
	pub status: CursorImageStatus,

	theme: CursorTheme,
	size: u32,

	cache: HashMap<CursorIcon, Option<XCursor>>,
}

impl Cursor {
	#[allow(clippy::new_without_default)]
	pub fn new() -> Self {
		let theme = CursorTheme::load("default");
		let size = 24;

		Cursor {
			status: CursorImageStatus::default_named(),

			theme,
			size,

			cache: HashMap::new(),
		}
	}
}

#[derive(Debug)]
struct Frame {
	image: Image,
}

impl Frame {
	fn new(image: Image) -> Self {
		Frame { image }
	}
}

#[derive(Debug)]
struct XCursor {
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
}

fn load_default_cursor() -> (MemoryRenderBuffer, Point<i32, Physical>) {
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

	let mem = MemoryRenderBuffer::from_slice(
		&icon.pixels_rgba,
		Fourcc::Argb8888,
		(icon.width as i32, icon.height as i32),
		2,
		Transform::Normal,
		None,
	);

	let hotspot = Point::from((icon.xhot as i32, icon.yhot as i32));

	(mem, hotspot)
}

pub struct CursorBuffer(Option<(MemoryRenderBuffer, Point<i32, Physical>)>);

impl CursorBuffer {
	pub const fn new() -> Self {
		CursorBuffer(None)
	}

	pub fn get(&mut self) -> (&MemoryRenderBuffer, &Point<i32, Physical>) {
		let (buf, hot) = self.0.get_or_insert_with(load_default_cursor);
		(buf, hot)
	}
}

impl Debug for CursorBuffer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("Cursor").field(&..).finish()
	}
}

impl Default for CursorBuffer {
	fn default() -> Self {
		Self::new()
	}
}
