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
    let mut client_ddata = ClientHandler::new(&client);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    let seat = client_ddata
        .globals
        .bind::<wayc::protocol::wl_seat::WlSeat, _, _>(&client.event_queue.handle(), 1..2, ())
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
    let mut client_ddata = ClientHandler::new(&client);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut ServerHandler).unwrap();

    let seat = client_ddata
        .globals
        .bind::<wayc::protocol::wl_seat::WlSeat, _, _>(&client.event_queue.handle(), 1..2, ())
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
                    .make_data::<wayc::protocol::wl_keyboard::WlKeyboard, _, ClientHandler>(()),
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

impl ClientHandler {
    fn new(client: &TestClient<ClientHandler>) -> ClientHandler {
        let globals = globals::GlobalList::new(&client.display, &client.event_queue.handle());
        ClientHandler { globals }
    }
}

impl AsMut<globals::GlobalList> for ClientHandler {
    fn as_mut(&mut self) -> &mut globals::GlobalList {
        &mut self.globals
    }
}

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
