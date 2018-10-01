#[macro_use]
extern crate wayland_client;

use wayland_client::{Display, GlobalManager};

use wayland_client::protocol::{wl_output, wl_seat};

// An example showcasing the capability of GlobalManager to handle
// dynamically created globals like wl_seat or wl_output, which can
// exist with multiplicity and created at any time

fn main() {
    let (display, mut event_queue) = Display::connect_to_env().unwrap();

    // We create a GlobalManager with a callback, that will be
    // advertised of any global creation or deletion
    let _globals = GlobalManager::new_with_cb(
        &display,
        // This macro generates a callback for auto-creating the globals
        // that interest us and calling our provided callbacks
        global_filter!(
            // Here we ask that all seats be automatically instantiated
            // with version 1 when advertised, and provide a callback that
            // will handle the created wl_seat to implement them
            //
            // NOTE: the type annotations are necessary because rustc's
            // inference is apparently not smart enough
            [wl_seat::WlSeat, 1, |seat: NewProxy<_>| {
                let mut seat_name = None;
                let mut caps = None;
                seat.implement(
                    move |event, _| {
                        use wayland_client::protocol::wl_seat::Event;
                        match event {
                            Event::Name { name } => {
                                seat_name = Some(name);
                            }
                            Event::Capabilities { capabilities } => {
                                // We *should* have received the "name" event first
                                caps = Some(capabilities);
                            }
                        }
                    },
                    (),
                )
            }],
            // Same thing with wl_output, but we require version 2
            [wl_output::WlOutput, 2, |output: NewProxy<_>| {
                let mut name = "<unknown>".to_owned();
                let mut modes = Vec::new();
                let mut scale = 1;
                output.implement(
                    move |event, _| {
                        use wayland_client::protocol::wl_output::Event;
                        match event {
                            Event::Geometry {
                                x,
                                y,
                                physical_width,
                                physical_height,
                                subpixel,
                                make,
                                model,
                                transform,
                            } => {
                                println!("New output: \"{} ({})\"", make, model);
                                println!(" -> physical dimensions {}x{}", physical_width, physical_height);
                                println!(" -> location in the compositor space: ({}, {})", x, y);
                                println!(" -> transform: {:?}", transform);
                                println!(" -> subpixel orientation: {:?}", subpixel);
                                name = format!("{} ({})", make, model);
                            }
                            Event::Mode {
                                flags,
                                width,
                                height,
                                refresh,
                            } => {
                                modes.push((flags, width, height, refresh));
                            }
                            Event::Scale { factor } => {
                                scale = factor;
                            }
                            Event::Done => {
                                println!("Modesetting information for output \"{}\"", name);
                                println!(" -> scaling factor: {}", scale);
                                println!(" -> mode list:");
                                for &(f, w, h, r) in &modes {
                                    println!(
                                        "   -> {}x{} @{}Hz (flags: [ {:?} ])",
                                        w,
                                        h,
                                        (r as f32) / 1000.0,
                                        f
                                    );
                                }
                            }
                        }
                    },
                    (),
                )
            }]
        ),
    );

    event_queue.sync_roundtrip().unwrap();
    event_queue.sync_roundtrip().unwrap();
    event_queue.sync_roundtrip().unwrap();
}
