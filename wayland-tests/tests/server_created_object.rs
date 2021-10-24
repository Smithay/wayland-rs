mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use ways::protocol::wl_data_device_manager::{
    Request as SDDMReq, WlDataDeviceManager as ServerDDMgr,
};
use ways::protocol::wl_data_offer::WlDataOffer as ServerDO;
use ways::protocol::wl_seat::WlSeat as ServerSeat;

use wayc::protocol::wl_data_device::Event as CDDEvt;
use wayc::protocol::wl_data_device_manager::WlDataDeviceManager as ClientDDMgr;
use wayc::protocol::wl_seat::WlSeat as ClientSeat;

#[test]
fn data_offer() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));
    server.display.create_global::<ServerDDMgr, _>(
        3,
        ways::Filter::new(move |(resource, version): (ways::Main<ServerDDMgr>, u32), _, _| {
            assert!(version == 3);
            resource.quick_assign(|_, request, _| match request {
                SDDMReq::GetDataDevice { id: ddevice, .. } => {
                    // create a data offer and send it
                    let offer = ddevice
                        .as_ref()
                        .client()
                        .unwrap()
                        .create_resource::<ServerDO>(ddevice.as_ref().version())
                        .unwrap();
                    // this must be the first server-side ID
                    assert_eq!(offer.as_ref().id(), 0xFF000000);
                    ddevice.data_offer(&offer);
                }
                _ => unimplemented!(),
            });
        }),
    );

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager.instantiate_exact::<ClientSeat>(1).unwrap();
    let ddmgr = manager.instantiate_exact::<ClientDDMgr>(3).unwrap();

    let received = Arc::new(Mutex::new(false));
    let received2 = received.clone();

    let ddevice = ddmgr.get_data_device(&seat);
    ddevice.quick_assign(move |_, evt, _| match evt {
        CDDEvt::DataOffer { id: doffer } => {
            let doffer = doffer.as_ref();
            assert!(doffer.version() == 3);
            // this must be the first server-side ID
            assert_eq!(doffer.id(), 0xFF000000);
            *received2.lock().unwrap() = true;
        }
        _ => unimplemented!(),
    });

    roundtrip(&mut client, &mut server).unwrap();

    assert!(*received.lock().unwrap());
}

#[test]
fn server_id_reuse() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));
    let srv_dd = Rc::new(RefCell::new(None));
    let srv_dd2 = srv_dd.clone();
    server.display.create_global::<ServerDDMgr, _>(
        3,
        ways::Filter::new(move |(resource, _): (ways::Main<ServerDDMgr>, u32), _, _| {
            let srv_dd3 = srv_dd2.clone();
            resource.quick_assign(move |_, req, _| {
                if let SDDMReq::GetDataDevice { id: ddevice, .. } = req {
                    *srv_dd3.borrow_mut() = Some(ddevice);
                }
            });
        }),
    );

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager.instantiate_exact::<ClientSeat>(1).unwrap();
    let ddmgr = manager.instantiate_exact::<ClientDDMgr>(3).unwrap();

    let offer = Rc::new(RefCell::new(None));
    let offer2 = offer.clone();

    let ddevice = ddmgr.get_data_device(&seat);

    ddevice.quick_assign(move |_, evt, _| match evt {
        CDDEvt::DataOffer { id: doffer } => {
            if let Some(old_offer) = ::std::mem::replace(&mut *offer2.borrow_mut(), Some(doffer)) {
                old_offer.destroy();
            }
        }
        _ => unimplemented!(),
    });

    roundtrip(&mut client, &mut server).unwrap();
    let ddevice = srv_dd.borrow().as_ref().unwrap().clone();

    // first send a data offer, it should be id 0xFF000000
    let offer1 = ddevice
        .as_ref()
        .client()
        .unwrap()
        .create_resource::<ServerDO>(ddevice.as_ref().version())
        .unwrap();
    offer1.quick_assign(|_, _, _| {});
    ddevice.data_offer(&offer1);
    roundtrip(&mut client, &mut server).unwrap();
    assert_eq!(offer.borrow().as_ref().unwrap().as_ref().id(), 0xFF000000);

    // then, send a second offer, it should be id 0xFF000001
    let offer2 = ddevice
        .as_ref()
        .client()
        .unwrap()
        .create_resource::<ServerDO>(ddevice.as_ref().version())
        .unwrap();
    offer2.quick_assign(|_, _, _| {});
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
        .unwrap();
    offer3.quick_assign(|_, _, _| {});
    ddevice.data_offer(&offer3);
    roundtrip(&mut client, &mut server).unwrap();
    assert_eq!(offer.borrow().as_ref().unwrap().as_ref().id(), 0xFF000000);
}

#[test]
fn server_created_race() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let server_do = Rc::new(RefCell::new(None));
    let server_do_2 = server_do.clone();
    server.display.create_global::<ServerDDMgr, _>(
        3,
        ways::Filter::new(move |(resource, _): (ways::Main<ServerDDMgr>, u32), _, _| {
            let server_do_3 = server_do_2.clone();
            resource.quick_assign(move |_, request, _| match request {
                SDDMReq::GetDataDevice { id: ddevice, .. } => {
                    // create a data offer and send it
                    let offer = ddevice
                        .as_ref()
                        .client()
                        .unwrap()
                        .create_resource::<ServerDO>(ddevice.as_ref().version())
                        .unwrap();
                    offer.quick_assign(|_, _, _| {});
                    // this must be the first server-side ID
                    ddevice.data_offer(&offer);
                    *server_do_3.borrow_mut() = Some(offer);
                }
                _ => unimplemented!(),
            });
        }),
    );

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager.instantiate_exact::<ClientSeat>(1).unwrap();
    let ddmgr = manager.instantiate_exact::<ClientDDMgr>(3).unwrap();

    let offer = Rc::new(RefCell::new(None));
    let offer2 = offer.clone();
    let received = Rc::new(Cell::new(0));
    let received_2 = received.clone();

    let ddevice = ddmgr.get_data_device(&seat);
    ddevice.quick_assign(move |_, evt, _| match evt {
        CDDEvt::DataOffer { id: doffer } => {
            let received_3 = received_2.clone();
            doffer.quick_assign(move |_, _, _| {
                received_3.set(received_3.get() + 1);
            });
            if let Some(old_offer) = ::std::mem::replace(&mut *offer2.borrow_mut(), Some(doffer)) {
                old_offer.destroy();
            }
        }
        _ => unimplemented!(),
    });

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
#[cfg(not(feature = "client_native"))]
#[test]
fn creation_destruction_race() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let server_dd = Rc::new(RefCell::new(Vec::new()));
    let server_dd_2 = server_dd.clone();
    server.display.create_global::<ServerDDMgr, _>(
        3,
        ways::Filter::new(move |(resource, _): (ways::Main<ServerDDMgr>, u32), _, _| {
            let server_dd_3 = server_dd_2.clone();
            resource.quick_assign(move |_, request, _| match request {
                SDDMReq::GetDataDevice { id: ddevice, .. } => {
                    ddevice.quick_assign(|_, _, _| {});
                    server_dd_3.borrow_mut().push(ddevice);
                }
                _ => unimplemented!(),
            });
        }),
    );

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager.instantiate_exact::<ClientSeat>(1).unwrap();
    let ddmgr = manager.instantiate_exact::<ClientDDMgr>(3).unwrap();

    let client_dd: Vec<_> = (0..2)
        .map(|_| {
            let ddevice = ddmgr.get_data_device(&seat);
            let mut offer = None;
            ddevice.quick_assign(move |_, evt, _| match evt {
                CDDEvt::DataOffer { id: doffer } => {
                    if let Some(old_offer) = ::std::mem::replace(&mut offer, Some(doffer)) {
                        old_offer.destroy();
                    }
                }
                _ => unimplemented!(),
            });
            ddevice
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
        .unwrap();
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
        .create_resource::<ServerDO>(server_dd.borrow()[1].as_ref().version())
        .unwrap();
    server_dd.borrow()[1].data_offer(&offer2);
    roundtrip(&mut client, &mut server).unwrap();

    offer2.offer("text".into());

    roundtrip(&mut client, &mut server).unwrap();
}

#[test]
fn creation_destruction_queue_dispatch_race() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(1, ways::Filter::new(|_: (_, _), _, _| {}));

    let server_dd = Rc::new(RefCell::new(Vec::new()));
    let server_dd_2 = server_dd.clone();
    server.display.create_global::<ServerDDMgr, _>(
        3,
        ways::Filter::new(move |(resource, _): (ways::Main<ServerDDMgr>, u32), _, _| {
            let server_dd_3 = server_dd_2.clone();
            resource.quick_assign(move |_, request, _| match request {
                SDDMReq::GetDataDevice { id: ddevice, .. } => {
                    server_dd_3.borrow_mut().push(ddevice);
                }
                _ => unimplemented!(),
            });
        }),
    );

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager.instantiate_exact::<ClientSeat>(1).unwrap();
    let ddmgr = manager.instantiate_exact::<ClientDDMgr>(3).unwrap();

    // this test is more subtle than the previous
    // here, the client destroys the data device while a data_offer
    // has been queued in the event queue but not yet dispatched to the handler.
    // the associated event should thus be dropped.

    let called_count = Rc::new(RefCell::new(0u32));

    let ddevice = ddmgr.get_data_device(&seat);
    let called_count2 = called_count.clone();
    ddevice.quick_assign(move |dd, evt, _| match evt {
        CDDEvt::DataOffer { .. } => {
            // destroy the data device after receiving the first offer
            dd.release();
            *called_count2.borrow_mut() += 1;
        }
        _ => unimplemented!(),
    });

    roundtrip(&mut client, &mut server).unwrap();

    // server sends two newid new sources
    let offer1 = server_dd.borrow()[0]
        .as_ref()
        .client()
        .unwrap()
        .create_resource::<ServerDO>(server_dd.borrow()[0].as_ref().version())
        .unwrap();
    server_dd.borrow()[0].data_offer(&offer1);
    let offer2 = server_dd.borrow()[0]
        .as_ref()
        .client()
        .unwrap()
        .create_resource::<ServerDO>(server_dd.borrow()[0].as_ref().version())
        .unwrap();
    server_dd.borrow()[0].data_offer(&offer2);

    roundtrip(&mut client, &mut server).unwrap();

    // now, the handler should only have been executed once
    assert_eq!(1, *called_count.borrow());
}
