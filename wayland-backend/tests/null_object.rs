use std::os::unix::net::UnixStream;
use wayland_backend::protocol::Message;

#[test]
fn rs_client_null_object_request() {
    use wayland_backend::rs::client::{Backend, InvalidId, ObjectId};

    let (sock, _sock2) = UnixStream::pair().unwrap();
    let backend = Backend::connect(sock).unwrap();
    let message = Message { sender_id: ObjectId::null(), opcode: 42, args: Default::default() };
    assert_eq!(backend.send_request(message, None, None), Err(InvalidId));
}

#[cfg(feature = "client_system")]
#[test]
fn sys_client_null_object_request() {
    use wayland_backend::sys::client::{Backend, InvalidId, ObjectId};

    let (sock, _sock2) = UnixStream::pair().unwrap();
    let backend = Backend::connect(sock).unwrap();
    let message = Message { sender_id: ObjectId::null(), opcode: 42, args: Default::default() };
    assert_eq!(backend.send_request(message, None, None), Err(InvalidId));
}

#[test]
fn rs_server_null_client_eq() {
    assert!(
        !wayland_backend::rs::server::ObjectId::null()
            .same_client_as(wayland_backend::rs::server::ObjectId::null())
    );
}

#[cfg(feature = "client_system")]
#[test]
fn sys_server_null_client_eq() {
    assert!(
        !wayland_backend::sys::server::ObjectId::null()
            .same_client_as(wayland_backend::sys::server::ObjectId::null())
    );
}
