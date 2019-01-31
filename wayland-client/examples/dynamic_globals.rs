#[macro_use]
extern crate wayland_client;

use wayland_client::{Display, GlobalManager};

use wayland_client::protocol::wl_output::{Mode, Subpixel, Transform, WlOutput};
use wayland_client::protocol::{wl_output, wl_seat};

// An example showcasing the capability of GlobalManager to handle
// dynamically created globals like wl_seat or wl_output, which can
// exist with multiplicity and created at any time

/// An event handler for wl_output.
///
/// We will use it to implement the wl_output globals.
struct OutputHandler {
    name: String,
    modes: Vec<(Mode, i32, i32, i32)>,
    scale: i32,
}

impl wl_output::EventHandler for OutputHandler {
    fn geometry(
        &mut self,
        _output: WlOutput,
        x: i32,
        y: i32,
        physical_width: i32,
        physical_height: i32,
        subpixel: Subpixel,
        make: String,
        model: String,
        transform: Transform,
    ) {
        println!("New output: \"{} ({})\"", make, model);
        println!(" -> physical dimensions {}x{}", physical_width, physical_height);
        println!(" -> location in the compositor space: ({}, {})", x, y);
        println!(" -> transform: {:?}", transform);
        println!(" -> subpixel orientation: {:?}", subpixel);
        self.name = format!("{} ({})", make, model);
    }

    fn mode(&mut self, _output: WlOutput, flags: Mode, width: i32, height: i32, refresh: i32) {
        self.modes.push((flags, width, height, refresh));
    }

    fn scale(&mut self, _output: WlOutput, factor: i32) {
        self.scale = factor;
    }

    fn done(&mut self, _output: WlOutput) {
        println!("Modesetting information for output \"{}\"", self.name);
        println!(" -> scaling factor: {}", self.scale);
        println!(" -> mode list:");
        for &(f, w, h, r) in &self.modes {
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
                seat.implement_closure(
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
                            _ => {}
                        }
                    },
                    (),
                )
            }],
            // Same thing with wl_output, but we require version 2
            [wl_output::WlOutput, 2, |output: NewProxy<_>| output.implement(
                OutputHandler {
                    name: "<unknown>".to_owned(),
                    modes: vec![],
                    scale: 1,
                },
                ()
            )]
        ),
    );

    event_queue.sync_roundtrip().unwrap();
    event_queue.sync_roundtrip().unwrap();
    event_queue.sync_roundtrip().unwrap();
}
