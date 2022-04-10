use wayland_client::{protocol::wl_registry, Connection, ConnectionHandle, Dispatch, QueueHandle};

struct AppData;

impl Dispatch<wl_registry::WlRegistry> for AppData {
    type UserData = ();

    fn event(
        &mut self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<AppData>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            println!("[{}] {} (v{})", name, interface, version);
        }
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();

    let display = conn.handle().display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let _registry = display.get_registry(&mut conn.handle(), &qh, ()).unwrap();

    println!("Advertized globals:");

    event_queue.sync_roundtrip(&mut AppData).unwrap();
}
