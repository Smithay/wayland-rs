#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, TestServer};

use ways::protocol::wl_compositor::WlCompositor as ServerCompositor;
use ways::protocol::wl_output::WlOutput as ServerOutput;
use ways::protocol::wl_shell::WlShell as ServerShell;

#[test]
fn simple_global() {
    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    let globals = client_ddata.globals.list();
    assert!(globals.len() == 1);
    assert_eq!(globals[0].name, 1);
    assert_eq!(globals[0].interface, "wl_compositor");
    assert_eq!(globals[0].version, 1);
}

#[test]
fn multi_versions() {
    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(4, ());
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(3, ());
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(2, ());
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    let globals = client_ddata.globals.list();
    assert!(globals.len() == 4);
    let mut seen = [false; 4];
    for global in globals {
        assert!(global.interface == "wl_compositor");
        seen[global.version as usize - 1] = true;
    }
    assert_eq!(seen, [true, true, true, true]);
}

#[test]
fn dynamic_global() {
    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();
    assert!(client_ddata.globals.list().len() == 1);

    server.display.handle().create_global::<ServerHandler, ServerShell, _>(1, ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();
    assert!(client_ddata.globals.list().len() == 2);

    let output = server.display.handle().create_global::<ServerHandler, ServerOutput, _>(1, ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();
    assert!(client_ddata.globals.list().len() == 3);

    server.display.handle().remove_global::<ServerHandler>(output);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();
    assert!(client_ddata.globals.list().len() == 2);
}

#[test]
#[should_panic]
fn wrong_version_create_global() {
    let server = TestServer::<ServerHandler>::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(42, ());
}

#[test]
fn wrong_global() {
    use wayc::protocol::wl_output::WlOutput;

    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(4, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    // instantiate a wrong global, this should kill the client
    // but currently does not fail on native_lib

    registry.bind::<WlOutput, _, _>(1, 1, &client.event_queue.handle(), ());

    assert!(roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).is_err());
}

#[test]
fn wrong_global_version() {
    use wayc::protocol::wl_compositor::WlCompositor;

    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    // instantiate a global with wrong version, this should kill the client

    registry.bind::<WlCompositor, _, _>(1, 2, &client.event_queue.handle(), ());

    assert!(roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).is_err());
}

#[test]
fn invalid_global_version() {
    use wayc::protocol::wl_compositor::WlCompositor;

    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    // instantiate a global with version 0, which is invalid this should kill the client

    registry.bind::<WlCompositor, _, _>(1, 0, &client.event_queue.handle(), ());

    assert!(roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).is_err());
}

#[test]
fn wrong_global_id() {
    use wayc::protocol::wl_compositor::WlCompositor;

    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    // instantiate a global with version 0, which is invalid this should kill the client

    registry.bind::<WlCompositor, _, _>(3, 1, &client.event_queue.handle(), ());

    assert!(roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).is_err());
}

#[test]
fn two_step_binding() {
    use wayc::protocol::{wl_compositor::WlCompositor, wl_output::WlOutput};

    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerCompositor, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    // add a new global while clients already exist
    server.display.handle().create_global::<ServerHandler, ServerOutput, _>(1, ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    client_ddata
        .globals
        .bind::<WlCompositor, _, _>(&client.event_queue.handle(), &registry, 1..2, ())
        .unwrap();

    client_ddata
        .globals
        .bind::<WlOutput, _, _>(&client.event_queue.handle(), &registry, 1..2, ())
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();
}

struct ServerHandler;

server_ignore_impl!(ServerHandler => [ServerCompositor, ServerShell, ServerOutput]);
server_ignore_global_impl!(ServerHandler => [ServerCompositor, ServerShell, ServerOutput]);

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

client_ignore_impl!(ClientHandler => [
    wayc::protocol::wl_compositor::WlCompositor,
    wayc::protocol::wl_shell::WlShell,
    wayc::protocol::wl_output::WlOutput
]);
