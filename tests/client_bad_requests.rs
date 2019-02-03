mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_seat::WlSeat as ServerSeat;

use wayc::protocol::wl_seat;

#[test]
fn constructor_dead() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(5, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager
        .instantiate_exact::<wl_seat::WlSeat, _>(5, |newp| newp.implement_dummy())
        .unwrap();

    seat.release();
    assert!(seat.get_pointer(|newp| newp.implement_dummy()).is_err());
}

#[test]
#[should_panic]
fn erroneous_send_constructor() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(5, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager
        .instantiate_exact::<wl_seat::WlSeat, _>(5, |newp| newp.implement_dummy())
        .unwrap();

    let _ = seat.as_ref().send_constructor::<wl_seat::WlSeat, _>(
        wl_seat::Request::Release,
        |newp| newp.implement_dummy(),
        None,
    );
}

#[test]
#[should_panic]
fn send_constructor_wrong_type() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(5, |_, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager
        .instantiate_exact::<wl_seat::WlSeat, _>(5, |newp| newp.implement_dummy())
        .unwrap();

    let _ = seat.as_ref().send_constructor::<wl_seat::WlSeat, _>(
        wl_seat::Request::GetPointer {
            id: seat.as_ref().child_placeholder(),
        },
        |newp| newp.implement_dummy(),
        None,
    );
}
