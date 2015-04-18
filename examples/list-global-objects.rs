extern crate wayland_client as wayland;

use wayland::interfaces::default_display;

fn main() {
    let display = default_display().expect("Unable to connect to Wayland server.");

    let registry = display.get_registry();

    display.sync_roundtrip();

    for (i , (s,v)) in registry.get_global_objects() {
        println!("id {} : {} ({})", i, s, v);
    }
}