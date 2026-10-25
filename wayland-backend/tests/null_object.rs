#![cfg_attr(windows, feature(windows_unix_domain_sockets))]

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use std::os::windows::net::UnixStream;
use wayland_backend::protocol::Message;

trait UnixStreamExt {
    fn pair_named(file: &str, line: u32) -> std::io::Result<(UnixStream, UnixStream)>;
}

#[cfg(unix)]
impl UnixStreamExt for UnixStream {
    fn pair_named(_file: &str, _line: u32) -> std::io::Result<(UnixStream, UnixStream)> {
        UnixStream::pair()
    }
}

#[cfg(windows)]
impl UnixStreamExt for UnixStream {
    fn pair_named(file: &str, line: u32) -> std::io::Result<(UnixStream, UnixStream)> {
        let file = std::path::Path::new(file);
        let file = file.file_stem().unwrap().display();
        let path = std::path::PathBuf::from(format!("stream-{file}-{line}.sock"));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        let rx_listener = std::os::windows::net::UnixListener::bind(&path)?;
        let tx = UnixStream::connect_addr(&rx_listener.local_addr()?)?;
        let (rx, _) = rx_listener.accept()?;
        Ok((tx, rx))
    }
}

#[test]
fn rs_client_null_object_request() {
    use wayland_backend::rs::client::{Backend, InvalidId, ObjectId};

    let (sock, _sock2) = UnixStream::pair_named(file!(), line!()).unwrap();
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
