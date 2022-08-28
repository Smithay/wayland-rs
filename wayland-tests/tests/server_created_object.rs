#[macro_use]
mod helpers;

use helpers::{globals, roundtrip, wayc, ways, TestServer};

use ways::protocol::wl_data_device::WlDataDevice as ServerDD;
use ways::protocol::wl_data_device_manager::{
    Request as SDDMReq, WlDataDeviceManager as ServerDDMgr,
};
use ways::protocol::wl_data_offer::WlDataOffer as ServerDO;
use ways::protocol::wl_seat::WlSeat as ServerSeat;
use ways::Resource;

use wayc::protocol::wl_data_device::Event as CDDEvt;
use wayc::protocol::wl_data_device_manager::WlDataDeviceManager as ClientDDMgr;
use wayc::protocol::wl_data_offer::WlDataOffer as ClientDO;
use wayc::protocol::wl_seat::WlSeat as ClientSeat;
use wayc::Proxy;

#[test]
fn data_offer() {
    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerSeat, _>(1, ());
    server.display.handle().create_global::<ServerHandler, ServerDDMgr, _>(3, ());
    let mut server_ddata = ServerHandler { data_device: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let seat = client_ddata
        .globals
        .bind::<ClientSeat, _, _>(&client.event_queue.handle(), &registry, 1..2, ())
        .unwrap();
    let ddmgr = client_ddata
        .globals
        .bind::<ClientDDMgr, _, _>(&client.event_queue.handle(), &registry, 3..4, ())
        .unwrap();

    ddmgr.get_data_device(&seat, &client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let server_dd = server_ddata.data_device.take().unwrap();
    let s_client = server.display.handle().get_client(server_dd.id()).unwrap();
    let offer = s_client
        .create_resource::<ServerDO, (), ServerHandler>(
            &server.display.handle(),
            server_dd.version(),
            (),
        )
        .unwrap();
    assert_eq!(offer.id().protocol_id(), 0xFF000000);
    server_dd.data_offer(&offer);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let client_do = client_ddata.data_offer.take().unwrap();
    assert_eq!(client_do.version(), 3);
    assert_eq!(client_do.id().protocol_id(), 0xFF000000);
}

#[test]
fn server_id_reuse() {
    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerSeat, _>(1, ());
    server.display.handle().create_global::<ServerHandler, ServerDDMgr, _>(3, ());
    let mut server_ddata = ServerHandler { data_device: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let seat = client_ddata
        .globals
        .bind::<ClientSeat, _, _>(&client.event_queue.handle(), &registry, 1..2, ())
        .unwrap();
    let ddmgr = client_ddata
        .globals
        .bind::<ClientDDMgr, _, _>(&client.event_queue.handle(), &registry, 3..4, ())
        .unwrap();

    ddmgr.get_data_device(&seat, &client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let server_dd = server_ddata.data_device.take().unwrap();
    let s_client = server.display.handle().get_client(server_dd.id()).unwrap();
    // Send a first data offer, ID should be 0xFF000000
    let offer = s_client
        .create_resource::<ServerDO, (), ServerHandler>(
            &server.display.handle(),
            server_dd.version(),
            (),
        )
        .unwrap();
    assert_eq!(offer.id().protocol_id(), 0xFF000000);
    server_dd.data_offer(&offer);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let client_offer = client_ddata.data_offer.take().unwrap();

    // Send a second data offer, ID should be 0xFF000001
    let offer = s_client
        .create_resource::<ServerDO, (), ServerHandler>(
            &server.display.handle(),
            server_dd.version(),
            (),
        )
        .unwrap();
    assert_eq!(offer.id().protocol_id(), 0xFF000001);
    server_dd.data_offer(&offer);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    // now the client destroys the offer

    client_offer.destroy();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    // Send a third data offer, server shoudl reuse id 0xFF000000
    let offer = s_client
        .create_resource::<ServerDO, (), ServerHandler>(
            &server.display.handle(),
            server_dd.version(),
            (),
        )
        .unwrap();
    assert_eq!(offer.id().protocol_id(), 0xFF000000);
    server_dd.data_offer(&offer);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let client_offer = client_ddata.data_offer.take().unwrap();

    assert_eq!(client_offer.id().protocol_id(), 0xFF000000);
}

#[test]
fn server_created_race() {
    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerSeat, _>(1, ());
    server.display.handle().create_global::<ServerHandler, ServerDDMgr, _>(3, ());
    let mut server_ddata = ServerHandler { data_device: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let seat = client_ddata
        .globals
        .bind::<ClientSeat, _, _>(&client.event_queue.handle(), &registry, 1..2, ())
        .unwrap();
    let ddmgr = client_ddata
        .globals
        .bind::<ClientDDMgr, _, _>(&client.event_queue.handle(), &registry, 3..4, ())
        .unwrap();

    ddmgr.get_data_device(&seat, &client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let server_dd = server_ddata.data_device.take().unwrap();
    let s_client = server.display.handle().get_client(server_dd.id()).unwrap();
    // Send a first data offer, ID should be 0xFF000000
    let offer = s_client
        .create_resource::<ServerDO, (), ServerHandler>(
            &server.display.handle(),
            server_dd.version(),
            (),
        )
        .unwrap();
    assert_eq!(offer.id().protocol_id(), 0xFF000000);
    server_dd.data_offer(&offer);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    offer.offer("text".into());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert_eq!(client_ddata.received.take().unwrap(), "text");

    // now the client will conccurently destroy the object as the server sends an event to it
    // this should not crash and the events to the zombie object should be silently dropped

    offer.offer("utf8".into());
    let client_do = client_ddata.data_offer.take().unwrap();
    client_do.destroy();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert!(client_ddata.received.is_none());
}

// this test currently crashes when using native_lib, this is a bug from the C lib
// see https://gitlab.freedesktop.org/wayland/wayland/issues/74
#[cfg(not(feature = "client_system"))]
#[test]
fn creation_destruction_race() {
    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerSeat, _>(1, ());
    server.display.handle().create_global::<ServerHandler, ServerDDMgr, _>(3, ());
    let mut server_ddata = ServerHandler { data_device: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let seat = client_ddata
        .globals
        .bind::<ClientSeat, _, _>(&client.event_queue.handle(), &registry, 1..2, ())
        .unwrap();
    let ddmgr = client_ddata
        .globals
        .bind::<ClientDDMgr, _, _>(&client.event_queue.handle(), &registry, 3..4, ())
        .unwrap();

    // client creates two data devices

    let client_dd1 = ddmgr.get_data_device(&seat, &client.event_queue.handle(), ());
    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();
    let s_dd1 = server_ddata.data_device.take().unwrap();

    ddmgr.get_data_device(&seat, &client.event_queue.handle(), ());
    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();
    let s_dd2 = server_ddata.data_device.take().unwrap();

    // server sends a newid event to dd1 while dd1 gets destroyed
    client_dd1.release();

    let s_client = server.display.handle().get_client(s_dd1.id()).unwrap();
    // Send a first NewID
    let offer1 = s_client
        .create_resource::<ServerDO, (), ServerHandler>(
            &server.display.handle(),
            s_dd1.version(),
            (),
        )
        .unwrap();
    s_dd1.data_offer(&offer1);
    // this message should not crash the client, even though it is send to
    // a object that has never been implemented
    offer1.offer("text".into());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert!(client_ddata.received.is_none());

    // server sends an other unrelated newid event
    let offer2 = s_client
        .create_resource::<ServerDO, (), ServerHandler>(
            &server.display.handle(),
            s_dd1.version(),
            (),
        )
        .unwrap();
    s_dd2.data_offer(&offer2);

    offer2.offer("utf8".into());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert_eq!(client_ddata.received.take().unwrap(), "utf8");
}

#[test]
fn creation_destruction_queue_dispatch_race() {
    let mut server = TestServer::new();
    server.display.handle().create_global::<ServerHandler, ServerSeat, _>(1, ());
    server.display.handle().create_global::<ServerHandler, ServerDDMgr, _>(3, ());
    let mut server_ddata = ServerHandler { data_device: None };

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler::new();

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let seat = client_ddata
        .globals
        .bind::<ClientSeat, _, _>(&client.event_queue.handle(), &registry, 1..2, ())
        .unwrap();
    let ddmgr = client_ddata
        .globals
        .bind::<ClientDDMgr, _, _>(&client.event_queue.handle(), &registry, 3..4, ())
        .unwrap();

    let client_dd = ddmgr.get_data_device(&seat, &client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let server_dd = server_ddata.data_device.take().unwrap();

    // this test is a subtler race than the previous
    // here, the client destroys the data device while a data_offer
    // has been queued in the event queue but not yet dispatched to the handler.
    //
    // In that case the wayland-client event queues dispatch the event anyway, but the receiver proxy will be dead

    let s_client = server.display.handle().get_client(server_dd.id()).unwrap();
    let offer = s_client
        .create_resource::<ServerDO, (), ServerHandler>(
            &server.display.handle(),
            server_dd.version(),
            (),
        )
        .unwrap();
    server_dd.data_offer(&offer);

    // Manually dispatch to cause the race
    server.display.flush_clients().unwrap();

    client.conn.prepare_read().unwrap().read().unwrap();

    client_dd.release();

    client.event_queue.dispatch_pending(&mut client_ddata).unwrap();

    // the zombie fallback is triggered
    assert!(client_ddata.received_dead);
}

struct ClientHandler {
    globals: globals::GlobalList,
    data_offer: Option<ClientDO>,
    received: Option<String>,
    received_dead: bool,
}

impl ClientHandler {
    fn new() -> ClientHandler {
        ClientHandler {
            globals: Default::default(),
            data_offer: None,
            received: None,
            received_dead: false,
        }
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
    ClientSeat,
    ClientDDMgr
]);

impl wayc::Dispatch<wayc::protocol::wl_data_device::WlDataDevice, ()> for ClientHandler {
    fn event(
        state: &mut Self,
        data_device: &wayc::protocol::wl_data_device::WlDataDevice,
        event: wayc::protocol::wl_data_device::Event,
        _: &(),
        conn: &wayc::Connection,
        _: &wayc::QueueHandle<Self>,
    ) {
        if conn.object_info(data_device.id()).is_err() {
            state.received_dead = true;
        }
        match event {
            CDDEvt::DataOffer { id } => {
                state.data_offer = Some(id);
            }
            _ => unimplemented!(),
        }
    }

    wayc::event_created_child!(ClientHandler, wayc::protocol::wl_data_device::WlDataDevice, [
        wayc::protocol::wl_data_device::EVT_DATA_OFFER_OPCODE => (ClientDO, ())
    ]);
}

impl wayc::Dispatch<ClientDO, ()> for ClientHandler {
    fn event(
        state: &mut Self,
        _: &ClientDO,
        event: wayc::protocol::wl_data_offer::Event,
        _: &(),
        _: &wayc::Connection,
        _: &wayc::QueueHandle<Self>,
    ) {
        match event {
            wayc::protocol::wl_data_offer::Event::Offer { mime_type } => {
                state.received = Some(mime_type);
            }
            _ => unimplemented!(),
        }
    }
}

struct ServerHandler {
    data_device: Option<ServerDD>,
}

server_ignore_impl!(ServerHandler => [
    ServerSeat,
    ServerDD,
    ServerDO
]);

server_ignore_global_impl!(ServerHandler => [
    ServerSeat,
    ServerDDMgr
]);

impl ways::Dispatch<ServerDDMgr, ()> for ServerHandler {
    fn request(
        state: &mut Self,
        _: &ways::Client,
        _: &ServerDDMgr,
        request: SDDMReq,
        _: &(),
        _: &ways::DisplayHandle,
        data_init: &mut ways::DataInit<'_, Self>,
    ) {
        match request {
            SDDMReq::GetDataDevice { id, .. } => {
                let dd = data_init.init(id, ());
                state.data_device = Some(dd);
            }
            _ => {
                unimplemented!()
            }
        }
    }
}
