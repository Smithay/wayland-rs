#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, TestServer};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[test]
fn client_user_data() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(1, ());
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_compositor::WlCompositor, _>(1, ());
    let mut server_ddata = ServerHandler {};

    let (s_client, mut client) = server.add_client_with_data(Arc::new(MyClientData {
        has_compositor: AtomicBool::new(false),
        has_output: AtomicBool::new(false),
    }));
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    // Instantiate the globals
    client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    {
        let cdata = s_client.get_data::<MyClientData>().unwrap();
        assert!(cdata.has_output.load(Ordering::SeqCst));
        assert!(!cdata.has_compositor.load(Ordering::SeqCst));
    }

    client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    {
        let cdata = s_client.get_data::<MyClientData>().unwrap();
        assert!(cdata.has_output.load(Ordering::SeqCst));
        assert!(cdata.has_compositor.load(Ordering::SeqCst));
    }
}

#[test]
fn client_credentials() {
    let mut server = TestServer::<()>::new();

    let (s_client, _) = server.add_client::<()>();

    let credentials = s_client.get_credentials(&server.display.handle());
    assert!(credentials.is_ok());
    assert_credentials(credentials.unwrap());
}

#[cfg(any(not(feature = "server_system"), not(target_os = "freebsd")))]
fn assert_credentials(credentials: ways::backend::Credentials) {
    assert!(credentials.pid != 0);
}

#[cfg(all(feature = "server_system", target_os = "freebsd"))]
fn assert_credentials(_credentials: ways::backend::Credentials) {
    // The current implementation of wl_client_get_credentials
    // will always return pid == 0 on freebsd
    // On recent versions this has been fixed with a freebsd
    // specific patch. Detecting if a patched version is used
    // is too complicated and this assert would just test the
    // native wayland-server library. So the assert is a no-op
    // for now.
    //
    // see: https://bugs.freebsd.org/bugzilla/show_bug.cgi?id=246189
}

struct ClientHandler {
    globals: globals::GlobalList,
}

impl ClientHandler {
    fn new() -> ClientHandler {
        ClientHandler { globals: Default::default() }
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

client_ignore_impl!(ClientHandler => [
    wayc::protocol::wl_output::WlOutput,
    wayc::protocol::wl_compositor::WlCompositor
]);

struct ServerHandler;

struct MyClientData {
    has_compositor: AtomicBool,
    has_output: AtomicBool,
}

impl ways::backend::ClientData for MyClientData {
    fn initialized(&self, _: wayland_backend::server::ClientId) {}
    fn disconnected(
        &self,
        _: wayland_backend::server::ClientId,
        _: wayland_backend::server::DisconnectReason,
    ) {
    }
}

server_ignore_impl!(ServerHandler => [
    ways::protocol::wl_output::WlOutput,
    ways::protocol::wl_compositor::WlCompositor
]);

impl ways::GlobalDispatch<ways::protocol::wl_output::WlOutput, ()> for ServerHandler {
    fn bind(
        _: &mut Self,
        _: &ways::DisplayHandle,
        client: &ways::Client,
        resource: ways::New<ways::protocol::wl_output::WlOutput>,
        _: &(),
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
        client.get_data::<MyClientData>().unwrap().has_output.store(true, Ordering::SeqCst);
    }
}

impl ways::GlobalDispatch<ways::protocol::wl_compositor::WlCompositor, ()> for ServerHandler {
    fn bind(
        _: &mut Self,
        _: &ways::DisplayHandle,
        client: &ways::Client,
        resource: ways::New<ways::protocol::wl_compositor::WlCompositor>,
        _: &(),
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
        client.get_data::<MyClientData>().unwrap().has_compositor.store(true, Ordering::SeqCst)
    }
}
