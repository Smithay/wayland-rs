extern crate wayland_client as wayland;

fn main() {
    let display = wayland::wayland::get_display().unwrap();
    let registry = display.get_registry();

    display.sync_roundtrip();
}