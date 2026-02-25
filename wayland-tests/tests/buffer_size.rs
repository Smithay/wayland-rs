use std::io::ErrorKind;

#[macro_use]
mod helpers;

use helpers::*;

#[test]
fn buffer_size() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_seat::WlSeat, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    let mut server_handler = ServerHandler::default();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_handler).unwrap();

    let seat = client_ddata
        .globals
        .bind::<wayc::protocol::wl_seat::WlSeat, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();

    let _pointer = seat.get_pointer(&client.event_queue.handle(), ());

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_handler).unwrap();

    // Send too many Wayland events to buffer
    let server_pointer = server_handler.pointer.as_ref().unwrap().clone();
    for _ in 0..10_000 {
        server_pointer.motion(0, 0., 0.);
    }

    // Verify we get `ConnectionReset` or `BrokenPipe` error
    // TODO: Why do we get one with one backend, and the other with the other backend?
    let res = roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_handler);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(matches!(err, wayc::backend::WaylandError::Io(_)));
    if let wayc::backend::WaylandError::Io(err) = err {
        assert!(matches!(err.kind(), ErrorKind::ConnectionReset | ErrorKind::BrokenPipe));
    }
}

#[cfg(feature = "libwayland_client_1_23_0")]
#[test]
fn buffer_size_client() {
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_seat::WlSeat, _>(1, ());

    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new() };

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    let mut server_handler = ServerHandler::default();

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_handler).unwrap();

    let seat = client_ddata
        .globals
        .bind::<wayc::protocol::wl_seat::WlSeat, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();

    let pointer = seat.get_pointer(&client.event_queue.handle(), ());

    // Send too many Wayland events to buffer for default server buffer
    for _ in 0..10_000 {
        pointer.set_cursor(0, None, 0, 0);
    }

    while let Err(wayc::backend::WaylandError::Io(err)) = client.conn.flush() {
        if err.kind() == ErrorKind::WouldBlock {
            server.answer(&mut server_handler);
        } else {
            break;
        }
    }
    server.answer(&mut server_handler);
    client.conn.flush().unwrap();

    // roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_handler).unwrap();
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

#[derive(Default)]
struct ServerHandler {
    pointer: Option<ways::protocol::wl_pointer::WlPointer>,
}

server_ignore_impl!(ServerHandler => [ways::protocol::wl_pointer::WlPointer]);

server_ignore_global_impl!(ServerHandler => [ways::protocol::wl_seat::WlSeat]);

impl ways::Dispatch<ways::protocol::wl_seat::WlSeat, ()> for ServerHandler {
    fn request(
        server_handler: &mut ServerHandler,
        _: &ways::Client,
        _: &ways::protocol::wl_seat::WlSeat,
        request: ways::protocol::wl_seat::Request,
        _: &(),
        _: &ways::DisplayHandle,
        data_init: &mut ways::DataInit<'_, ServerHandler>,
    ) {
        match request {
            ways::protocol::wl_seat::Request::GetPointer { id } => {
                server_handler.pointer = Some(data_init.init(id, ()));
            }
            _ => unreachable!(),
        }
    }
}
