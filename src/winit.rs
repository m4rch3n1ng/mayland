use crate::{Data, MayState};
use smithay::{
	backend::{
		input::InputEvent,
		renderer::{
			damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
			gles::GlesRenderer,
		},
		winit::{self, WinitEvent},
	},
	output::{Mode, Output, PhysicalProperties, Subpixel},
	reexports::calloop::EventLoop,
	utils::{Rectangle, Transform},
};
use std::time::Duration;

pub fn init(calloop: &mut EventLoop<Data>, data: &mut Data) {
	let display_handle = &mut data.display_handle;
	let state = &mut data.state;

	let (mut backend, winit) = winit::init().unwrap();

	let mode = Mode {
		size: backend.window_size(),
		refresh: 60_000,
	};

	let output = Output::new(
		"winit".to_owned(),
		PhysicalProperties {
			size: (0, 0).into(),
			subpixel: Subpixel::Unknown,
			make: "may".to_owned(),
			model: "winit".to_owned(),
		},
	);

	state.space.map_output(&output, (0, 0));

	let _global = output.create_global::<MayState>(display_handle);
	output.change_current_state(
		Some(mode),
		Some(Transform::Flipped180),
		None,
		Some((0, 0).into()),
	);
	output.set_preferred(mode);

	let mut damage_tracker = OutputDamageTracker::from_output(&output);

	std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);
	std::env::set_var("GDK_BACKEND", "wayland");

	calloop
		.handle()
		.insert_source(winit, move |event, (), data| {
			let display = &mut data.display_handle;
			let state = &mut data.state;

			match event {
				WinitEvent::Resized { size, .. } => {
					output.change_current_state(
						Some(Mode {
							size,
							refresh: 60_000,
						}),
						None,
						None,
						None,
					);
				}
				WinitEvent::Redraw => {
					let size = backend.window_size();
					let damage = Rectangle::from_loc_and_size((0, 0), size);

					backend.bind().unwrap();
					smithay::desktop::space::render_output::<
						_,
						WaylandSurfaceRenderElement<GlesRenderer>,
						_,
						_,
					>(
						&output,
						backend.renderer(),
						1.0,
						0,
						[&state.space],
						&[],
						&mut damage_tracker,
						[0.1, 0.1, 0.1, 1.0],
					)
					.unwrap();
					backend.submit(Some(&[damage])).unwrap();

					state.space.elements().for_each(|window| {
						window.send_frame(
							&output,
							state.start_time.elapsed(),
							Some(Duration::ZERO),
							|_, _| Some(output.clone()),
						);
					});

					state.space.refresh();
					state.popups.cleanup();
					let _ = display.flush_clients();

					// Ask for redraw to schedule new frame.
					backend.window().request_redraw();
				}
				WinitEvent::CloseRequested => {
					state.loop_signal.stop();
				}
				event => println!("event {:?}", event),
			}
		})
		.unwrap();
}
