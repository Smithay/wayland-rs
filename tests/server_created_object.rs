mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use std::sync::{Arc, Mutex};

use ways::protocol::wl_data_device::WlDataDevice as ServerDD;
use ways::protocol::wl_data_device_manager::{
    Request as SDDMReq, RequestHandler as ServerDDMHandler, WlDataDeviceManager as ServerDDMgr,
};
use ways::protocol::wl_data_offer::WlDataOffer as ServerDO;
use ways::protocol::wl_data_source::WlDataSource as ServerDS;
use ways::protocol::wl_seat::WlSeat as ServerSeat;
use ways::NewResource;

use wayc::protocol::wl_data_device::{
    Event as CDDEvt, EventHandler as ClientDDHandler, WlDataDevice as ClientDD,
};
use wayc::protocol::wl_data_device_manager::WlDataDeviceManager as ClientDDMgr;
use wayc::protocol::wl_data_offer::WlDataOffer as ClientDO;
use wayc::protocol::wl_seat::WlSeat as ClientSeat;
use wayc::protocol::wl_surface::WlSurface as ClientSurface;
use wayc::NewProxy;

#[test]
fn data_offer() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(1, |_, _| {});
    server
        .display
        .create_global::<ServerDDMgr, _>(3, |new_resource, version| {
            assert!(version == 3);
            new_resource.implement_closure(
                |request, _| match request {
                    SDDMReq::GetDataDevice { id, .. } => {
                        let ddevice = id.implement_dummy();
                        // create a data offer and send it
                        let offer = ddevice
                            .as_ref()
                            .client()
                            .unwrap()
                            .create_resource::<ServerDO>(ddevice.as_ref().version())
                            .unwrap()
                            .implement_dummy();
                        // this must be the first server-side ID
                        assert_eq!(offer.as_ref().id(), 0xFF000000);
                        ddevice.data_offer(&offer)
                    }
                    _ => unimplemented!(),
                },
                None::<fn(_)>,
                (),
            );
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager
        .instantiate_auto::<ClientSeat, _>(|newseat| newseat.implement_dummy())
        .unwrap();
    let ddmgr = manager
        .instantiate_auto::<ClientDDMgr, _>(|newddmgr| newddmgr.implement_dummy())
        .unwrap();

    let received = Arc::new(Mutex::new(false));
    let received2 = received.clone();

    ddmgr
        .get_data_device(&seat, move |newdd| {
            newdd.implement_closure(
                move |evt, _| match evt {
                    CDDEvt::DataOffer { id } => {
                        let doffer = id.implement_dummy();
                        let doffer = doffer.as_ref();
                        assert!(doffer.version() == 3);
                        // this must be the first server-side ID
                        assert_eq!(doffer.id(), 0xFF000000);
                        *received2.lock().unwrap() = true;
                    }
                    _ => unimplemented!(),
                },
                (),
            )
        })
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    assert!(*received.lock().unwrap());
}

#[test]
fn data_offer_trait_impls() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(1, |_, _| {});

    struct ServerHandler;
    impl ServerDDMHandler for ServerHandler {
        fn get_data_device(&mut self, _ddmgr: ServerDDMgr, id: NewResource<ServerDD>, _seat: ServerSeat) {
            let ddevice = id.implement_dummy();
            // create a data offer and send it
            let offer = ddevice
                .as_ref()
                .client()
                .unwrap()
                .create_resource::<ServerDO>(ddevice.as_ref().version())
                .unwrap()
                .implement_dummy();
            // this must be the first server-side ID
            assert_eq!(offer.as_ref().id(), 0xFF000000);
            ddevice.data_offer(&offer)
        }

        fn create_data_source(&mut self, _ddmgr: ServerDDMgr, _id: NewResource<ServerDS>) {
            unimplemented!()
        }
    }

    server
        .display
        .create_global::<ServerDDMgr, _>(3, |new_resource, version| {
            assert!(version == 3);
            new_resource.implement(ServerHandler, None::<fn(_)>, ());
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager
        .instantiate_auto::<ClientSeat, _>(|newseat| newseat.implement_dummy())
        .unwrap();
    let ddmgr = manager
        .instantiate_auto::<ClientDDMgr, _>(|newddmgr| newddmgr.implement_dummy())
        .unwrap();

    let received = Arc::new(Mutex::new(false));
    let received2 = received.clone();

    struct ClientHandler {
        received: Arc<Mutex<bool>>,
    }

    impl ClientDDHandler for ClientHandler {
        fn data_offer(&mut self, _dd: ClientDD, id: NewProxy<ClientDO>) {
            let doffer = id.implement_dummy();
            let doffer = doffer.as_ref();
            assert!(doffer.version() == 3);
            // this must be the first server-side ID
            assert_eq!(doffer.id(), 0xFF000000);
            *self.received.lock().unwrap() = true;
        }

        fn enter(
            &mut self,
            _dd: ClientDD,
            _serial: u32,
            _surface: ClientSurface,
            _x: f64,
            _y: f64,
            _id: Option<ClientDO>,
        ) {
            unimplemented!()
        }

        fn leave(&mut self, _dd: ClientDD) {
            unimplemented!()
        }

        fn motion(&mut self, _dd: ClientDD, _time: u32, _x: f64, _y: f64) {
            unimplemented!()
        }

        fn drop(&mut self, _dd: ClientDD) {
            unimplemented!()
        }

        fn selection(&mut self, _dd: ClientDD, _id: Option<ClientDO>) {
            unimplemented!()
        }
    }

    ddmgr
        .get_data_device(&seat, move |newdd| {
            newdd.implement(ClientHandler { received: received2 }, ())
        })
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    assert!(*received.lock().unwrap());
}
