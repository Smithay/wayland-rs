extern crate wayland_client;

use wayland_client::Display;

use wayland_client::protocol::wl_display::RequestsTrait;
use wayland_client::protocol::wl_registry::Events as RegistryEvents;

fn main() {
    let (display, mut event_queue) = Display::connect_to_env().unwrap();

    let registry = display
        .get_registry()
        .unwrap()
        .implement((), |_, evt, _| match evt {
            RegistryEvents::Global {
                name,
                interface,
                version,
            } => {
                println!(
                    "New global with id {} and version {} of interface '{}'.",
                    name, version, interface
                );
            }
            _ => {}
        });

    event_queue.sync_roundtrip().unwrap();
    event_queue.sync_roundtrip().unwrap();
}
