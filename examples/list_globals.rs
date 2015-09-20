extern crate wayland_client as wayland;

use wayland::{Proxy, EventIterator};

fn main() {
    let mut display = wayland::wayland::get_display().unwrap();

    let evt_iter = EventIterator::new();
    display.set_evt_iterator(&evt_iter);

    let mut registry = display.get_registry();
    registry.set_evt_iterator(&evt_iter);

    display.sync_roundtrip();

    for evt in evt_iter {
        println!("{:?}", evt);
    }
}