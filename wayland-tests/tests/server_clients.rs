#[macro_use]
mod helpers;

use helpers::{roundtrip, wayc, ways, TestServer};

use std::sync::{Arc, Mutex};

#[test]
fn client_user_data() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput>(1, ());
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_compositor::WlCompositor>(1, ());
    let mut server_ddata = ServerHandler {};

    let (s_client, mut client) = server.add_client_with_data(Arc::new(Mutex::new(MyClientData {
        has_compositor: false,
        has_output: false,
    })));
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ()).unwrap();

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
        let guard = s_client.get_data::<Mutex<MyClientData>>().unwrap().lock().unwrap();
        assert!(guard.has_output);
        assert!(!guard.has_compositor);
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
        let guard = s_client.get_data::<Mutex<MyClientData>>().unwrap().lock().unwrap();
        assert!(guard.has_output);
        assert!(guard.has_compositor);
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
    globals: wayc::globals::GlobalList,
}

impl ClientHandler {
    fn new() -> ClientHandler {
        ClientHandler { globals: Default::default() }
    }
}

impl AsMut<wayc::globals::GlobalList> for ClientHandler {
    fn as_mut(&mut self) -> &mut wayc::globals::GlobalList {
        &mut self.globals
    }
}

wayc::delegate_dispatch!(ClientHandler:
    [wayc::protocol::wl_registry::WlRegistry: ()] => wayc::globals::GlobalList
);

client_ignore_impl!(ClientHandler => [
    wayc::protocol::wl_output::WlOutput,
    wayc::protocol::wl_compositor::WlCompositor
]);

struct ServerHandler;

struct MyClientData {
    has_compositor: bool,
    has_output: bool,
}

impl ways::backend::ClientData<ServerHandler> for Mutex<MyClientData> {
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

impl ways::GlobalDispatch<ways::protocol::wl_output::WlOutput> for ServerHandler {
    type GlobalData = ();
    fn bind(
        &mut self,
        _: &ways::DisplayHandle,
        client: &ways::Client,
        resource: ways::New<ways::protocol::wl_output::WlOutput>,
        _: &Self::GlobalData,
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
        client.get_data::<Mutex<MyClientData>>().unwrap().lock().unwrap().has_output = true;
    }
}

impl ways::GlobalDispatch<ways::protocol::wl_compositor::WlCompositor> for ServerHandler {
    type GlobalData = ();
    fn bind(
        &mut self,
        _: &ways::DisplayHandle,
        client: &ways::Client,
        resource: ways::New<ways::protocol::wl_compositor::WlCompositor>,
        _: &Self::GlobalData,
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
        client.get_data::<Mutex<MyClientData>>().unwrap().lock().unwrap().has_compositor = true;
    }
}
