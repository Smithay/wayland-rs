use std::sync::Arc;

use wayland_client::{oneshot_sink, protocol::wl_registry, Connection};

fn main() {
    let cx = Connection::connect_to_env().unwrap();

    let display = cx.handle().display();

    let registry_data = oneshot_sink!(wl_registry::WlRegistry, move |_cx, _registry, event| {
        eprintln!("{:?}", event);
    });

    let _registry = display.get_registry(&mut cx.handle(), Some(registry_data)).unwrap();

    cx.roundtrip().unwrap();
}
