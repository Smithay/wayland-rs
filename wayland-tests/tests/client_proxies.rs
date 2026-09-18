use wayland_tests::{
    TestServer, globals, roundtrip, server_ignore_global_impl, server_ignore_impl, wayc, ways,
};

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

    let registry =
        client.display.get_registry(&client.event_queue.handle(), globals::GlobalListData);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let compositor1 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..=1,
            0,
        )
        .unwrap();

    let compositor2 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..=1,
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

    let registry =
        client.display.get_registry(&client.event_queue.handle(), globals::GlobalListData);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let compositor1 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..=1,
            0xDEADBEEFusize,
        )
        .unwrap();

    let compositor2 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..=1,
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

    let registry =
        client.display.get_registry(&client.event_queue.handle(), globals::GlobalListData);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let output = client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..=3,
            wayc::NoopIgnore,
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

    let registry =
        client.display.get_registry(&client.event_queue.handle(), globals::GlobalListData);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let output = client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..=3,
            wayc::NoopIgnore,
        )
        .unwrap();
    let compositor = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..=1,
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

impl ways::GlobalDispatch<ways::protocol::wl_output::WlOutput, ServerHandler> for () {
    fn bind(
        &self,
        state: &mut ServerHandler,
        _: &ways::DisplayHandle,
        _: &ways::Client,
        output: ways::New<ways::protocol::wl_output::WlOutput>,
        data_init: &mut ways::DataInit<'_, ServerHandler>,
    ) {
        let output = data_init.init(output, ());
        state.output = Some(output);
    }
}

impl ways::Dispatch<ways::protocol::wl_compositor::WlCompositor, ServerHandler> for () {
    fn request(
        &self,
        state: &mut ServerHandler,
        _: &ways::Client,
        _: &ways::protocol::wl_compositor::WlCompositor,
        request: ways::protocol::wl_compositor::Request,
        dhandle: &ways::DisplayHandle,
        data_init: &mut ways::DataInit<'_, ServerHandler>,
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

impl wayc::Dispatch<wayc::protocol::wl_compositor::WlCompositor, ClientHandler> for usize {
    fn event(
        &self,
        _: &mut ClientHandler,
        _: &wayc::protocol::wl_compositor::WlCompositor,
        _: wayc::protocol::wl_compositor::Event,
        _: &wayc::Connection,
        _: &wayc::QueueHandle<ClientHandler>,
    ) {
    }
}

impl wayc::Dispatch<wayc::protocol::wl_surface::WlSurface, ClientHandler> for () {
    fn event(
        &self,
        state: &mut ClientHandler,
        _: &wayc::protocol::wl_surface::WlSurface,
        event: wayc::protocol::wl_surface::Event,
        conn: &wayc::Connection,
        _: &wayc::QueueHandle<ClientHandler>,
    ) {
        if let wayc::protocol::wl_surface::Event::Enter { output } = event {
            assert!(conn.get_object_data(output.id()).is_err());
            state.entered = true;
        } else {
            panic!("Unexpected event: {event:?}");
        }
    }
}
