#[macro_use]
mod helpers;

use helpers::*;
use wayc::Proxy;

#[test]
fn constructor_dead() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_seat::WlSeat, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    let seat = client_ddata
        .globals
        .bind::<wayc::protocol::wl_seat::WlSeat, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();

    seat.release();

    assert!(seat.get_pointer(&client.event_queue.handle(), ()).id().is_null());
}

#[test]
fn send_constructor_wrong_type() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_seat::WlSeat, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    let seat = client_ddata
        .globals
        .bind::<wayc::protocol::wl_seat::WlSeat, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();

    let id = client
        .conn
        .send_request(
            &seat,
            wayc::protocol::wl_seat::Request::GetPointer {},
            Some(
                client
                    .event_queue
                    .handle()
                    .make_data::<wayc::protocol::wl_keyboard::WlKeyboard, _>(()),
            ),
        )
        .unwrap();

    // The ID points to a wl_pointer, so trying to make a wl_keyboard from it should fail
    assert!(wayc::protocol::wl_keyboard::WlKeyboard::from_id(&client.conn, id).is_err())
}

/*
 * Client handler
 */

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
    wayc::protocol::wl_seat::WlSeat,
    wayc::protocol::wl_pointer::WlPointer,
    wayc::protocol::wl_keyboard::WlKeyboard
]);

/*
 * Server handler
 */

struct ServerHandler;

server_ignore_impl!(ServerHandler => [ways::protocol::wl_pointer::WlPointer, ways::protocol::wl_seat::WlSeat]);

server_ignore_global_impl!(ServerHandler => [ways::protocol::wl_seat::WlSeat]);
