use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

struct AppData;

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        &mut self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            println!("[{}] {} (v{})", name, interface, version);
        }
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let _registry = display.get_registry(&qh, ()).unwrap();

    println!("Advertized globals:");

    event_queue.sync_roundtrip(&mut AppData).unwrap();
}
