mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use ways::protocol::wl_seat::WlSeat as ServerSeat;

use wayc::protocol::{wl_keyboard, wl_seat};

#[test]
fn constructor_dead() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(5, |_, _, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager.instantiate_exact::<wl_seat::WlSeat>(5).unwrap();

    seat.release();
    assert!(!seat.get_pointer().as_ref().is_alive());
}

#[test]
#[should_panic]
fn send_constructor_wrong_type() {
    let mut server = TestServer::new();
    server.display.create_global::<ServerSeat, _>(5, |_, _, _| {});

    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    roundtrip(&mut client, &mut server).unwrap();

    let seat = manager.instantiate_exact::<wl_seat::WlSeat>(5).unwrap();

    let _ = seat
        .as_ref()
        .send::<wl_keyboard::WlKeyboard>(wl_seat::Request::GetPointer {}, None);
}
