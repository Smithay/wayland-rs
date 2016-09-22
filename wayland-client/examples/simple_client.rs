#[macro_use] extern crate wayland_client;

use wayland_client::EventQueueHandle;
use wayland_client::protocol::wl_registry;

struct MyHandler;

impl wl_registry::Handler for MyHandler {
    fn global(&mut self, _: &mut EventQueueHandle, _: &wl_registry::WlRegistry, name: u32, interface: String, version: u32) {
        println!("global {}: {} (version {})", name, interface, version);
    }
    fn global_remove(&mut self, _: &mut EventQueueHandle, _: &wl_registry::WlRegistry, _: u32) {
        println!("global_remove");
    }
}

declare_handler!(MyHandler, wl_registry::Handler, wl_registry::WlRegistry);

fn main() {
    let (display, mut event_queue) = match wayland_client::default_connect() {
        Ok(ret) => ret,
        Err(e) => panic!("Cannot connect to wayland server: {:?}", e)
    };

    event_queue.add_handler(MyHandler);
    let registry = display.get_registry().expect("Registry cannot be destroyed!?");
    event_queue.register::<_, MyHandler>(&registry, 0);

    event_queue.sync_roundtrip().unwrap();
}
