#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, TestServer};

#[test]
fn destroyed_object_in_arg() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_compositor::WlCompositor, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::default();
    let registry = client.display.get_registry(&client.event_queue.handle(), ());
    let qh = client.event_queue.handle();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    let compositor = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(&qh, &registry, 1..=1, ())
        .unwrap();
    let surface = compositor.create_surface(&qh, ());
    let region = compositor.create_region(&qh, ());
    region.destroy();
    surface.set_input_region(Some(&region));

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();
}

struct ServerHandler;

impl ways::Dispatch<ways::protocol::wl_compositor::WlCompositor, ()> for ServerHandler {
    fn request(
        _state: &mut Self,
        _: &ways::Client,
        _: &ways::protocol::wl_compositor::WlCompositor,
        request: ways::protocol::wl_compositor::Request,
        _: &(),
        _: &ways::DisplayHandle,
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        match request {
            ways::protocol::wl_compositor::Request::CreateSurface { id } => {
                let _surface = data_init.init(id, ());
            }
            ways::protocol::wl_compositor::Request::CreateRegion { id } => {
                let _region = data_init.init(id, ());
            }
            ways::protocol::wl_compositor::Request::Release => {}
            _ => {
                unimplemented!()
            }
        }
    }
}

server_ignore_impl!(ServerHandler => [
    ways::protocol::wl_region::WlRegion,
    ways::protocol::wl_surface::WlSurface
]);
server_ignore_global_impl!(ServerHandler => [
    ways::protocol::wl_compositor::WlCompositor
]);

#[derive(Default)]
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
wayc::delegate_noop!(ClientHandler: wayc::protocol::wl_compositor::WlCompositor);
wayc::delegate_noop!(ClientHandler: ignore wayc::protocol::wl_surface::WlSurface);
wayc::delegate_noop!(ClientHandler: wayc::protocol::wl_region::WlRegion);
