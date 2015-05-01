extern crate wayland_client as wayland;

use wayland::core::default_display;

fn main() {
    let display = default_display().expect("Unable to connect to Wayland server.");

    let registry = display.get_registry();

    // let the registry fetch its events
    display.sync_roundtrip();

    for (i, s, v) in registry.get_global_objects() {
        println!("id {} : {} ({})", i, s, v);
    }

    let outputs = registry.get_outputs();

    // let the outputs fetch their events
    display.sync_roundtrip();

    println!("");
    println!("Available outputs:");

    for output in &outputs {
        println!("- {}: {} {:?}",
            output.manufacturer(), output.model(), output.dimensions());
        for m in output.modes() {
            let p = if m.is_preferred() { 'p' } else { ' ' };
            let c = if m.is_current() { 'c' } else { ' ' };
            println!("   - [{}{}] {}x{} ({})", p, c, m.width, m.height, m.refresh);
        }
    }
}