mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use std::sync::{Arc, Mutex};

use ways::protocol::wl_data_device::Event as SDDEvt;
use ways::protocol::wl_data_device_manager::{Request as SDDMReq, WlDataDeviceManager as ServerDDMgr};
use ways::protocol::wl_data_offer::WlDataOffer as ServerDO;
use ways::protocol::wl_seat::WlSeat as ServerSeat;
use ways::NewResource;

use wayc::protocol::wl_data_device::Event as CDDEvt;
use wayc::protocol::wl_data_device_manager::{RequestsTrait, WlDataDeviceManager as ClientDDMgr};
use wayc::protocol::wl_seat::WlSeat as ClientSeat;

#[test]
fn data_offer() {
    let mut server = TestServer::new();
    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<ServerSeat, _>(&loop_token, 1, |_, _| {});
    server.display.create_global::<ServerDDMgr, _>(
        &loop_token,
        3,
        |version, new_resource: NewResource<_>| {
            assert!(version == 3);
            new_resource.implement(
                |request, _| match request {
                    SDDMReq::GetDataDevice { id, .. } => {
                        let ddevice = id.implement(|_, _| {}, None::<fn(_, _)>);
                        // create a data offer and send it
                        let offer = ddevice
                            .client()
                            .unwrap()
                            .create_resource::<ServerDO>(ddevice.version())
                            .unwrap()
                            .implement(|_, _| {}, None::<fn(_, _)>);
                        ddevice.send(SDDEvt::DataOffer { id: offer })
                    }
                    _ => unimplemented!(),
                },
                None::<fn(_, _)>,
            );
        },
    );

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager
        .instantiate_auto::<ClientSeat, _>(|newseat| newseat.implement(|_, _| {}))
        .unwrap();
    let ddmgr = manager
        .instantiate_auto::<ClientDDMgr, _>(|newddmgr| newddmgr.implement(|_, _| {}))
        .unwrap();

    let received = Arc::new(Mutex::new(false));
    let received2 = received.clone();

    ddmgr
        .get_data_device(&seat, move |newdd| {
            newdd.implement(move |evt, _| match evt {
                CDDEvt::DataOffer { id } => {
                    let doffer = id.implement(|_, _| {});
                    assert!(doffer.version() == 3);
                    *received2.lock().unwrap() = true;
                }
                _ => unimplemented!(),
            })
        })
        .unwrap();

    roundtrip(&mut client, &mut server).unwrap();

    assert!(*received.lock().unwrap());
}
