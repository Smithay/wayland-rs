mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use std::cell::{Cell, RefCell};
use std::rc::Rc;
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
        .instantiate_exact::<ClientSeat, _>(1, |newseat| newseat.implement_dummy())
        .unwrap();
    let ddmgr = manager
        .instantiate_exact::<ClientDDMgr, _>(3, |newddmgr| newddmgr.implement_dummy())
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
        .instantiate_exact::<ClientSeat, _>(1, |newseat| newseat.implement_dummy())
        .unwrap();
    let ddmgr = manager
        .instantiate_exact::<ClientDDMgr, _>(3, |newddmgr| newddmgr.implement_dummy())
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

#[test]
fn server_id_reuse() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(1, |_, _| {});
    let srv_dd = Rc::new(RefCell::new(None));
    let srv_dd2 = srv_dd.clone();
    server
        .display
        .create_global::<ServerDDMgr, _>(3, move |new_resource, _| {
            let srv_dd3 = srv_dd2.clone();
            new_resource.implement_closure(
                move |req, _| {
                    if let SDDMReq::GetDataDevice { id, .. } = req {
                        let ddevice = id.implement_dummy();
                        *srv_dd3.borrow_mut() = Some(ddevice);
                    }
                },
                None::<fn(_)>,
                (),
            );
        });

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager
        .instantiate_exact::<ClientSeat, _>(1, |newseat| newseat.implement_dummy())
        .unwrap();
    let ddmgr = manager
        .instantiate_exact::<ClientDDMgr, _>(3, |newddmgr| newddmgr.implement_dummy())
        .unwrap();

    let offer = Rc::new(RefCell::new(None));
    let offer2 = offer.clone();

    ddmgr
        .get_data_device(&seat, move |newdd| {
            newdd.implement_closure(
                move |evt, _| match evt {
                    CDDEvt::DataOffer { id } => {
                        let doffer = id.implement_dummy();
                        if let Some(old_offer) = ::std::mem::replace(&mut *offer2.borrow_mut(), Some(doffer))
                        {
                            old_offer.destroy();
                        }
                    }
                    _ => unimplemented!(),
                },
                (),
            )
        })
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();
    let ddevice = srv_dd.borrow().as_ref().unwrap().clone();

    // first send a data offer, it should be id 0xFF000000
    let offer1 = ddevice
        .as_ref()
        .client()
        .unwrap()
        .create_resource::<ServerDO>(ddevice.as_ref().version())
        .unwrap()
        .implement_dummy();
    ddevice.data_offer(&offer1);
    roundtrip(&mut client, &mut server).unwrap();
    assert_eq!(offer.borrow().as_ref().unwrap().as_ref().id(), 0xFF000000);

    // then, send a second offer, it should be id 0xFF000001
    let offer2 = ddevice
        .as_ref()
        .client()
        .unwrap()
        .create_resource::<ServerDO>(ddevice.as_ref().version())
        .unwrap()
        .implement_dummy();
    ddevice.data_offer(&offer2);
    roundtrip(&mut client, &mut server).unwrap();
    assert_eq!(offer.borrow().as_ref().unwrap().as_ref().id(), 0xFF000001);

    // a roundtrip so that the message of destruction of the first offer reaches the server
    roundtrip(&mut client, &mut server).unwrap();

    // then send a third, given the first has been destroyed in the meantime, it should reuse
    // the first id 0xFF000000
    let offer3 = ddevice
        .as_ref()
        .client()
        .unwrap()
        .create_resource::<ServerDO>(ddevice.as_ref().version())
        .unwrap()
        .implement_dummy();
    ddevice.data_offer(&offer3);
    roundtrip(&mut client, &mut server).unwrap();
    assert_eq!(offer.borrow().as_ref().unwrap().as_ref().id(), 0xFF000000);
}

#[test]
fn server_created_race() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(1, |_, _| {});

    let server_do = Rc::new(RefCell::new(None));
    let server_do_2 = server_do.clone();
    server
        .display
        .create_global::<ServerDDMgr, _>(3, move |new_resource, _| {
            let server_do_3 = server_do_2.clone();
            new_resource.implement_closure(
                move |request, _| match request {
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
                        ddevice.data_offer(&offer);
                        *server_do_3.borrow_mut() = Some(offer);
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
        .instantiate_exact::<ClientSeat, _>(1, |newseat| newseat.implement_dummy())
        .unwrap();
    let ddmgr = manager
        .instantiate_exact::<ClientDDMgr, _>(3, |newddmgr| newddmgr.implement_dummy())
        .unwrap();

    let offer = Rc::new(RefCell::new(None));
    let offer2 = offer.clone();
    let received = Rc::new(Cell::new(0));
    let received_2 = received.clone();

    ddmgr
        .get_data_device(&seat, move |newdd| {
            newdd.implement_closure(
                move |evt, _| match evt {
                    CDDEvt::DataOffer { id } => {
                        let received_3 = received_2.clone();
                        let doffer = id.implement_closure(
                            move |_, _| {
                                received_3.set(received_3.get() + 1);
                            },
                            (),
                        );
                        if let Some(old_offer) = ::std::mem::replace(&mut *offer2.borrow_mut(), Some(doffer))
                        {
                            old_offer.destroy();
                        }
                    }
                    _ => unimplemented!(),
                },
                (),
            )
        })
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    server_do.borrow().as_ref().unwrap().offer("text".into());
    roundtrip(&mut client, &mut server).unwrap();
    assert_eq!(received.get(), 1);

    // the server has send an event that the client has received, all is good
    // now, the server will send more events but the client will conccurently
    // destroy the object, this should not crash and the events to the zombie object
    // should be silently dropped
    server_do.borrow().as_ref().unwrap().offer("utf8".into());
    offer.borrow().as_ref().unwrap().destroy();
    roundtrip(&mut client, &mut server).unwrap();
    assert_eq!(received.get(), 1);
}

// this test currently crashes when using native_lib, this is a bug from the C lib
// see https://gitlab.freedesktop.org/wayland/wayland/issues/74
#[cfg(not(feature = "native_lib"))]
#[test]
fn creation_destruction_race() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(1, |_, _| {});

    let server_dd = Rc::new(RefCell::new(Vec::new()));
    let server_dd_2 = server_dd.clone();
    server
        .display
        .create_global::<ServerDDMgr, _>(3, move |new_resource, _| {
            let server_dd_3 = server_dd_2.clone();
            new_resource.implement_closure(
                move |request, _| match request {
                    SDDMReq::GetDataDevice { id, .. } => {
                        let ddevice = id.implement_dummy();
                        server_dd_3.borrow_mut().push(ddevice);
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
        .instantiate_exact::<ClientSeat, _>(1, |newseat| newseat.implement_dummy())
        .unwrap();
    let ddmgr = manager
        .instantiate_exact::<ClientDDMgr, _>(3, |newddmgr| newddmgr.implement_dummy())
        .unwrap();

    let client_dd: Vec<_> = (0..2)
        .map(|_| {
            ddmgr
                .get_data_device(&seat, move |newdd| {
                    let mut offer = None;
                    newdd.implement_closure(
                        move |evt, _| match evt {
                            CDDEvt::DataOffer { id } => {
                                let doffer = id.implement_dummy();
                                if let Some(old_offer) = ::std::mem::replace(&mut offer, Some(doffer)) {
                                    old_offer.destroy();
                                }
                            }
                            _ => unimplemented!(),
                        },
                        (),
                    )
                })
                .unwrap()
        })
        .collect();

    roundtrip(&mut client, &mut server).unwrap();

    // server sends a newid event to dd1 while dd1 gets destroyed
    client_dd[0].release();
    let offer1 = server_dd.borrow()[0]
        .as_ref()
        .client()
        .unwrap()
        .create_resource::<ServerDO>(server_dd.borrow()[0].as_ref().version())
        .unwrap()
        .implement_dummy();
    server_dd.borrow()[0].data_offer(&offer1);
    roundtrip(&mut client, &mut server).unwrap();
    // this message should not crash the client, even though it is send to
    // a object that has never been implemented
    offer1.offer("text".into());

    // server sends an other unrelated newid event
    let offer2 = server_dd.borrow()[1]
        .as_ref()
        .client()
        .unwrap()
        .create_resource::<ServerDO>(server_dd.borrow()[0].as_ref().version())
        .unwrap()
        .implement_dummy();
    server_dd.borrow()[1].data_offer(&offer2);
    roundtrip(&mut client, &mut server).unwrap();

    offer2.offer("text".into());

    roundtrip(&mut client, &mut server).unwrap();
}
