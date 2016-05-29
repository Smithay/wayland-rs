extern crate wayland_client as wayland;

use wayland::Event;
use wayland::wayland::{WaylandProtocolEvent, WlRegistryEvent};

fn main() {
    let (display, mut event_iter) = match wayland::get_display() {
        Ok(d) => d,
        Err(e) => panic!("Unable to connect to a wayland compositor: {:?}", e)
    };

    // Get the registry, to generate the events advertizing global objects
    let _registry = display.get_registry();

    // Roundtrip, to make sure all event are dispatched to us
    event_iter.sync_roundtrip().unwrap();

    while let Ok(Some(evt)) = event_iter.next_event_dispatch() {
        match evt {
            // Global advertising events are `WlRegistryEvent::Global`
            Event::Wayland(WaylandProtocolEvent::WlRegistry(
                _, WlRegistryEvent::Global(name, interface, version)
            )) => {
                println!("[{:>4}] {} (version {})", name, interface, version);
            },
            // ignore everything else
            _ => {}
        }
    }
}