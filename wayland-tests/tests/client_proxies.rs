#[macro_use]
mod helpers;

use helpers::{roundtrip, wayc, ways, TestServer};

use ways::Resource;

use wayc::Proxy;

#[test]
fn proxy_equals() {
    let mut server = TestServer::new();
    server.display.create_global::<ways::protocol::wl_compositor::WlCompositor>(1, ());
    let mut server_ddata = ServerHandler { output: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client
        .display
        .get_registry(&mut client.conn.handle(), &client.event_queue.handle(), ())
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let compositor1 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _>(
            &mut client.conn.handle(),
            &client.event_queue.handle(),
            &registry,
            1..2,
            0,
        )
        .unwrap();

    let compositor2 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _>(
            &mut client.conn.handle(),
            &client.event_queue.handle(),
            &registry,
            1..2,
            0,
        )
        .unwrap();

    let compositor3 = compositor1.clone();

    assert!(compositor1 == compositor3);
    assert!(compositor1 != compositor2);
    assert!(compositor2 != compositor3);
}

#[test]
fn proxy_user_data() {
    let mut server = TestServer::new();
    server.display.create_global::<ways::protocol::wl_compositor::WlCompositor>(1, ());
    let mut server_ddata = ServerHandler { output: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client
        .display
        .get_registry(&mut client.conn.handle(), &client.event_queue.handle(), ())
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let compositor1 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _>(
            &mut client.conn.handle(),
            &client.event_queue.handle(),
            &registry,
            1..2,
            0xDEADBEEFusize,
        )
        .unwrap();

    let compositor2 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _>(
            &mut client.conn.handle(),
            &client.event_queue.handle(),
            &registry,
            1..2,
            0xBADC0FFEusize,
        )
        .unwrap();

    let compositor3 = compositor1.clone();

    assert!(compositor1.data::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor2.data::<usize>() == Some(&0xBADC0FFE));
    assert!(compositor3.data::<usize>() == Some(&0xDEADBEEF));
    assert!(compositor1.data::<u32>() == None);
}

#[test]
fn dead_proxies() {
    let mut server = TestServer::new();
    server.display.create_global::<ways::protocol::wl_output::WlOutput>(3, ());
    let mut server_ddata = ServerHandler { output: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client
        .display
        .get_registry(&mut client.conn.handle(), &client.event_queue.handle(), ())
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let output = client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _>(
            &mut client.conn.handle(),
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let output2 = output.clone();

    assert!(output == output2);
    assert!(client.conn.handle().object_info(output.id()).is_ok());
    assert!(client.conn.handle().object_info(output2.id()).is_ok());

    // kill the output
    output.release(&mut client.conn.handle());

    // dead proxies are still equal
    assert!(output == output2);
    assert!(client.conn.handle().object_info(output.id()).is_err());
    assert!(client.conn.handle().object_info(output2.id()).is_err());
}

#[test]
fn dead_object_argument() {
    let mut server = TestServer::new();
    server.display.create_global::<ways::protocol::wl_compositor::WlCompositor>(1, ());
    server.display.create_global::<ways::protocol::wl_output::WlOutput>(3, ());
    let mut server_ddata = ServerHandler { output: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client
        .display
        .get_registry(&mut client.conn.handle(), &client.event_queue.handle(), ())
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let output = client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _>(
            &mut client.conn.handle(),
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();
    let compositor = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _>(
            &mut client.conn.handle(),
            &client.event_queue.handle(),
            &registry,
            1..2,
            0,
        )
        .unwrap();

    compositor.create_surface(&mut client.conn.handle(), &client.event_queue.handle(), ()).unwrap();
    output.release(&mut client.conn.handle());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert!(client_ddata.entered);
}

struct ServerHandler {
    output: Option<ways::protocol::wl_output::WlOutput>,
}

impl ways::GlobalDispatch<ways::protocol::wl_output::WlOutput> for ServerHandler {
    type GlobalData = ();
    fn bind(
        &mut self,
        _: &mut ways::DisplayHandle<'_>,
        _: &ways::Client,
        output: ways::New<ways::protocol::wl_output::WlOutput>,
        _: &(),
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        let output = data_init.init(output, ());
        self.output = Some(output);
    }
}

impl ways::Dispatch<ways::protocol::wl_compositor::WlCompositor> for ServerHandler {
    type UserData = ();
    fn request(
        &mut self,
        _: &ways::Client,
        _: &ways::protocol::wl_compositor::WlCompositor,
        request: ways::protocol::wl_compositor::Request,
        _: &(),
        dhandle: &mut ways::DisplayHandle<'_>,
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        if let ways::protocol::wl_compositor::Request::CreateSurface { id } = request {
            let surface = data_init.init(id, ());
            let output = self.output.clone().unwrap();
            assert!(dhandle.object_info(output.id()).is_ok());
            surface.enter(dhandle, &output);
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
    globals: wayc::globals::GlobalList,
    entered: bool,
}

impl ClientHandler {
    fn new() -> ClientHandler {
        ClientHandler { globals: Default::default(), entered: false }
    }
}

impl AsMut<wayc::globals::GlobalList> for ClientHandler {
    fn as_mut(&mut self) -> &mut wayc::globals::GlobalList {
        &mut self.globals
    }
}

wayc::delegate_dispatch!(ClientHandler:
    [wayc::protocol::wl_registry::WlRegistry] => wayc::globals::GlobalList
);

impl wayc::Dispatch<wayc::protocol::wl_compositor::WlCompositor> for ClientHandler {
    type UserData = usize;

    fn event(
        &mut self,
        _: &wayc::protocol::wl_compositor::WlCompositor,
        _: wayc::protocol::wl_compositor::Event,
        _: &usize,
        _: &mut wayc::ConnectionHandle,
        _: &wayc::QueueHandle<Self>,
    ) {
    }
}

impl wayc::Dispatch<wayc::protocol::wl_surface::WlSurface> for ClientHandler {
    type UserData = ();

    fn event(
        &mut self,
        _: &wayc::protocol::wl_surface::WlSurface,
        event: wayc::protocol::wl_surface::Event,
        _: &(),
        connhandle: &mut wayc::ConnectionHandle,
        _: &wayc::QueueHandle<Self>,
    ) {
        if let wayc::protocol::wl_surface::Event::Enter { output } = event {
            assert!(connhandle.get_object_data(output.id()).is_err());
            self.entered = true;
        } else {
            panic!("Unexpected event: {:?}", event);
        }
    }
}

client_ignore_impl!(ClientHandler => [
    wayc::protocol::wl_output::WlOutput
]);
