#[macro_use]
extern crate wayland_client;

use wayland_client::{Display, GlobalManager};

use wayland_client::protocol::wl_display::RequestsTrait;
use wayland_client::protocol::{wl_output, wl_seat};

// An example showcasing the capability of GlobalManager to handle
// dynamically created globals like wl_seat or wl_output, which can
// exist with multiplicity and created at any time

fn main() {
    let (display, mut event_queue) = Display::connect_to_env().unwrap();

    // We create a GlobalManager with a callback, that will be
    // advertized of any global creation or deletion
    let _globals = GlobalManager::new_with_cb(
        display.get_registry().unwrap(),
        // This macro generates a callback for auto-creating the globals
        // that interest us and calling our provided callbacks
        global_filter!(
            // Here we ask that all seats be automatically instanciated
            // with version 1 when advertized, and provide a callback that
            // will handle the created wl_seat to implement them
            //
            // NOTE: the type annotations are necessary because rustc's
            // inference is apparently not smart enough
            [wl_seat::WlSeat, 1, |seat: Result<NewProxy<_>, _>, ()| {
                // here seat is a result, as failure can happen if the server
                // advertized an lower version than we requested.
                // This cannot happen here as we requested version 1
                let seat = seat.unwrap();
                let mut seat_name = None;
                let mut caps = None;
                seat.implement(move |event, _| {
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
                    if let (&Some(ref caps), &Some(ref name)) = (&caps, &seat_name) {
                        println!("New seat \"{}\" with capabilities [ {:?} ]", name, caps);
                    }
                });
            }],
            // Same thing with wl_output, but we require version 2
            [wl_output::WlOutput, 2, |output: Result<NewProxy<_>, _>, ()| {
                let output = output.unwrap();
                let mut name = "<unknown>".to_owned();
                let mut modes = Vec::new();
                let mut scale = 1;
                output.implement(move |event, _| {
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
                });
            }]
        ),
    );

    event_queue.sync_roundtrip().unwrap();
    event_queue.sync_roundtrip().unwrap();
    event_queue.sync_roundtrip().unwrap();
}
