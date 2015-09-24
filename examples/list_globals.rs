extern crate wayland_client as wayland;

use wayland::{Proxy, EventIterator, Event};
use wayland::wayland::{WaylandProtocolEvent, WlRegistryEvent};

fn main() {
    let mut display = match wayland::wayland::get_display() {
        Some(d) => d,
        None => panic!("Unable to connect to a wayland compositor.")
    };

    // Create an event iterator and assign it to the display
    // so that it is automatically inherited by all created objects
    let evt_iter = EventIterator::new();
    display.set_evt_iterator(&evt_iter);

    // Get the registry, to generate the events advertizing global objects
    let _registry = display.get_registry();

    // Roundtrip, to make sure all event are dispatched to us
    display.sync_roundtrip().unwrap();

    for evt in evt_iter {
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