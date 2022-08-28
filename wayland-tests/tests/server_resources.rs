#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, TestServer};

use ways::{
    protocol::{wl_compositor, wl_output},
    Resource,
};

use wayc::protocol::wl_output::WlOutput as ClientOutput;

#[test]
fn resource_equals() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(3, ());
    let mut server_ddata = ServerHandler { outputs: Vec::new() };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    // create two outputs
    client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();
    client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert!(server_ddata.outputs.len() == 2);
    assert!(server_ddata.outputs[0] != server_ddata.outputs[1]);

    let cloned = server_ddata.outputs[0].clone();
    assert!(server_ddata.outputs[0] == cloned);

    assert!(server_ddata.outputs[0].id().same_client_as(&server_ddata.outputs[1].id()));
}

#[test]
fn resource_user_data() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(3, ());
    let mut server_ddata = ServerHandler { outputs: Vec::new() };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    // create two outputs
    client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();
    client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert_eq!(server_ddata.outputs[0].data::<UData>().unwrap().0, 1000);
    assert_eq!(server_ddata.outputs[1].data::<UData>().unwrap().0, 1001);
    let cloned = server_ddata.outputs[0].clone();
    assert_eq!(cloned.data::<UData>().unwrap().0, 1000);
}

#[test]
fn dead_resources() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(3, ());
    let mut server_ddata = ServerHandler { outputs: Vec::new() };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    // create two outputs
    let client_output_1 = client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();
    client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert!(server.display.handle().get_object_data(server_ddata.outputs[0].id()).is_ok());
    assert!(server.display.handle().get_object_data(server_ddata.outputs[1].id()).is_ok());

    let cloned = server_ddata.outputs[0].clone();

    client_output_1.release();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert!(server.display.handle().get_object_data(server_ddata.outputs[0].id()).is_err());
    assert!(server.display.handle().get_object_data(server_ddata.outputs[1].id()).is_ok());
    assert!(server.display.handle().get_object_data(cloned.id()).is_err());
}

#[test]
fn get_resource() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_output::WlOutput, _>(3, ());
    let mut server_ddata = ServerHandler { outputs: Vec::new() };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    // create an outputs
    client_ddata
        .globals
        .bind::<wayc::protocol::wl_output::WlOutput, _, _>(
            &client.event_queue.handle(),
            &registry,
            3..4,
            (),
        )
        .unwrap();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    // try to retrieve the resource
    // its id should be 3 (1 is wl_display and 2 is wl_registry)
    let client = server.display.handle().get_client(server_ddata.outputs[0].id()).unwrap();
    // wrong interface fails
    assert!(client
        .object_from_protocol_id::<wl_compositor::WlCompositor>(&server.display.handle(), 3)
        .is_err());
    // wrong id fails
    assert!(client
        .object_from_protocol_id::<wl_output::WlOutput>(&server.display.handle(), 4)
        .is_err());
    // but this suceeds
    assert!(client
        .object_from_protocol_id::<wl_output::WlOutput>(&server.display.handle(), 3)
        .is_ok());
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

client_ignore_impl!(ClientHandler => [ClientOutput]);

struct ServerHandler {
    outputs: Vec<wl_output::WlOutput>,
}

impl ways::GlobalDispatch<wl_output::WlOutput, ()> for ServerHandler {
    fn bind(
        state: &mut Self,
        _: &ways::DisplayHandle,
        _: &ways::Client,
        output: ways::New<ways::protocol::wl_output::WlOutput>,
        _: &(),
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        let output = data_init.init(output, UData(1000 + state.outputs.len()));
        state.outputs.push(output);
    }
}

struct UData(usize);

impl ways::Dispatch<wl_output::WlOutput, UData> for ServerHandler {
    fn request(
        _: &mut Self,
        _: &ways::Client,
        _: &wl_output::WlOutput,
        _: wl_output::Request,
        _: &UData,
        _: &ways::DisplayHandle,
        _: &mut ways::DataInit<'_, Self>,
    ) {
    }
}
