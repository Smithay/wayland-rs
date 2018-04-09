extern crate wayland_client;

use wayland_client::{Display, GlobalManager};

use wayland_client::protocol::wl_display::RequestsTrait;

// A minimal example printing the list of globals advertized by the server and
// then exiting

fn main() {
    // Connect to the server
    let (display, mut event_queue) = Display::connect_to_env().unwrap();

    // We use the GlobalManager convenience provided by the crate, it covers
    // most classic use cases and avoids us the trouble to manually implement
    // the registry
    let globals = GlobalManager::new(display.get_registry().unwrap());

    // A roundtrip synchronization to make sure the server received our registry
    // creation and sent us the global list
    event_queue.sync_roundtrip().unwrap();

    // Print the list
    for (id, interface, version) in globals.list() {
        println!("{}: {} (version {})", id, interface, version);
    }
}
