#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, TestServer};

use ways::Resource;

use wayc::Proxy;

#[test]
fn proxy_equals() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_compositor::WlCompositor, _>(1, ());
    let mut server_ddata = ServerHandler { output: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let compositor1 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            0,
        )
        .unwrap();

    let compositor2 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            0,
        )
        .unwrap();

    #[allow(clippy::redundant_clone)]
    let compositor3 = compositor1.clone();

    assert!(compositor1 == compositor3);
    assert!(compositor1 != compositor2);
    assert!(compositor2 != compositor3);
}

#[test]
fn proxy_user_data() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_compositor::WlCompositor, _>(1, ());
    let mut server_ddata = ServerHandler { output: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let compositor1 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            0xDEADBEEFusize,
        )
        .unwrap();

    let compositor2 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            0xBADC0FFEusize,
        )
        .unwrap();

    #[allow(clippy::redundant_clone)]
    let compositor3 = compositor1.clone();

    assert!(compositor1.data::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor2.data::<usize>() == Some(&0xBADC0FFE));
    assert!(compositor3.data::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor1.data::<u32>().is_none());
}

#[test]
fn dead_proxies() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(3, ());
    let mut server_ddata = ServerHandler { output: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let output = client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    #[allow(clippy::redundant_clone)]
    let output2 = output.clone();

    assert!(output == output2);
    assert!(client.conn.object_info(output.id()).is_ok());
    assert!(client.conn.object_info(output2.id()).is_ok());

    // kill the output
    output.release();

    // dead proxies are still equal
    assert!(output == output2);
    assert!(client.conn.object_info(output.id()).is_err());
    assert!(client.conn.object_info(output2.id()).is_err());
}

#[test]
fn dead_object_argument() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_compositor::WlCompositor, _>(1, ());
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(3, ());
    let mut server_ddata = ServerHandler { output: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let output = client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();
    let compositor = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            0,
        )
        .unwrap();

    compositor.create_surface(&client.event_queue.handle(), ());
    output.release();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert!(client_ddata.entered);
}

struct ServerHandler {
    output: Option<ways::protocol::wl_output::WlOutput>,
}

impl ways::GlobalDispatch<ways::protocol::wl_output::WlOutput, ()> for ServerHandler {
    fn bind(
        state: &mut Self,
        _: &ways::DisplayHandle,
        _: &ways::Client,
        output: ways::New<ways::protocol::wl_output::WlOutput>,
        _: &(),
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        let output = data_init.init(output, ());
        state.output = Some(output);
    }
}

impl ways::Dispatch<ways::protocol::wl_compositor::WlCompositor, ()> for ServerHandler {
    fn request(
        state: &mut Self,
        _: &ways::Client,
        _: &ways::protocol::wl_compositor::WlCompositor,
        request: ways::protocol::wl_compositor::Request,
        _: &(),
        dhandle: &ways::DisplayHandle,
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        if let ways::protocol::wl_compositor::Request::CreateSurface { id } = request {
            let surface = data_init.init(id, ());
            let output = state.output.clone().unwrap();
            assert!(dhandle.object_info(output.id()).is_ok());
            surface.enter(&output);
        }
    }
}

server_ignore_impl!(ServerHandler => [
    ways::protocol::wl_output::WlOutput,
    ways::protocol::wl_surface::WlSurface
]);

server_ignore_global_impl!(ServerHandler => [
    ways::protocol::wl_compositor::WlCompositor
]);

struct ClientHandler {
    globals: globals::GlobalList,
    entered: bool,
}

impl ClientHandler {
    fn new() -> ClientHandler {
        ClientHandler { globals: Default::default(), entered: false }
    }
}

impl AsMut<globals::GlobalList> for ClientHandler {
    fn as_mut(&mut self) -> &mut globals::GlobalList {
        &mut self.globals
    }
}

wayc::delegate_dispatch!(ClientHandler:
    [wayc::protocol::wl_registry::WlRegistry: ()] => globals::GlobalList
);

impl wayc::Dispatch<wayc::protocol::wl_compositor::WlCompositor, usize> for ClientHandler {
    fn event(
        _: &mut Self,
        _: &wayc::protocol::wl_compositor::WlCompositor,
        _: wayc::protocol::wl_compositor::Event,
        _: &usize,
        _: &wayc::Connection,
        _: &wayc::QueueHandle<Self>,
    ) {
    }
}

impl wayc::Dispatch<wayc::protocol::wl_surface::WlSurface, ()> for ClientHandler {
    fn event(
        state: &mut Self,
        _: &wayc::protocol::wl_surface::WlSurface,
        event: wayc::protocol::wl_surface::Event,
        _: &(),
        conn: &wayc::Connection,
        _: &wayc::QueueHandle<Self>,
    ) {
        if let wayc::protocol::wl_surface::Event::Enter { output } = event {
            assert!(conn.get_object_data(output.id()).is_err());
            state.entered = true;
        } else {
            panic!("Unexpected event: {event:?}");
        }
    }
}

client_ignore_impl!(ClientHandler => [
    wayc::protocol::wl_output::WlOutput
]);
