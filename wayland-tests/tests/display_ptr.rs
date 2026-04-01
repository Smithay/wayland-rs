#![cfg(all(feature = "client_system", feature = "libwayland_client_1_23"))]

use wayc::Proxy;
use wayland_tests::{globals, wayc, TestServer};

#[test]
fn client_objectid_display_ptr() {
    let mut server = TestServer::<ServerHandler>::new();

    let (_s_client, client) = server.add_client::<ClientHandler>();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    registry.id().display_ptr().unwrap();
}

struct ClientHandler {
    globals: globals::GlobalList,
}

impl AsMut<globals::GlobalList> for ClientHandler {
    fn as_mut(&mut self) -> &mut globals::GlobalList {
        &mut self.globals
    }
}

wayc::delegate_dispatch!(ClientHandler:
    [wayc::protocol::wl_registry::WlRegistry: ()] => globals::GlobalList
);

struct ServerHandler;
