#[macro_use] extern crate wayland_client;

use wayland_client::event_queue::EventQueueHandle;
use wayland_client::protocol::wl_registry;

struct MyHandler;

impl wl_registry::Handler for MyHandler {
    fn global(&mut self, _: &mut EventQueueHandle, _: &wl_registry::WlRegistry, name: u32, interface: String, version: u32) {
        println!("global");
    }
    fn global_remove(&mut self, _: &mut EventQueueHandle, _: &wl_registry::WlRegistry, name: u32) {
        println!("global_remove");
    }
}

declare_handler!(MyHandler, wl_registry::Handler, wl_registry::WlRegistry);

fn main() {
}
