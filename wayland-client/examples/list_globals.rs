use std::sync::Arc;

use wayland_client::{protocol::wl_registry, quick_sink, Connection};

fn main() {
    let cx = Connection::connect_to_env().unwrap();

    let display = cx.handle().display();

    let registry_data = quick_sink!(wl_registry::WlRegistry, move |_cx, (_registry, event)| {
        if let wl_registry::Event::Global { name, interface, version } = event {
            eprintln!("[{}] {} (v{})", name, interface, version);
        }
    });

    let _registry = display.get_registry(&mut cx.handle(), Some(registry_data)).unwrap();

    eprintln!("Advertized globals:");

    cx.roundtrip().unwrap();
}
