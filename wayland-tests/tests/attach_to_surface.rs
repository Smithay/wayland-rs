extern crate tempfile;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};

#[macro_use]
mod helpers;

use helpers::{roundtrip, wayc, ways, TestServer};

use wayc::protocol::wl_shm::Format;

use ways::protocol::wl_buffer::WlBuffer as ServerBuffer;

#[test]
fn attach_null() {
    // Server setup
    //
    let mut server = TestServer::new();
    server.display.handle().create_global::<ways::protocol::wl_compositor::WlCompositor>(1, ());
    let mut server_ddata = ServerHandler { buffer_found: None, fd_found: None };

    // Client setup
    //
    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: wayc::globals::GlobalList::new() };

    let registry = client
        .display
        .get_registry(&mut client.conn.handle(), &client.event_queue.handle(), ())
        .unwrap();

    // Initial sync
    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let compositor = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _>(
            &mut client.conn.handle(),
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();
    let surface = compositor
        .create_surface(&mut client.conn.handle(), &client.event_queue.handle(), ())
        .unwrap();
    surface.attach(&mut client.conn.handle(), None, 0, 0);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    assert_eq!(server_ddata.buffer_found, Some(None));
}

#[test]
fn attach_buffer() {
    // Server setup
    //
    let mut server = TestServer::new();
    server.display.handle().create_global::<ways::protocol::wl_compositor::WlCompositor>(1, ());
    server.display.handle().create_global::<ways::protocol::wl_shm::WlShm>(1, ());
    let mut server_ddata = ServerHandler { buffer_found: None, fd_found: None };

    // Client setup
    //
    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: wayc::globals::GlobalList::new() };

    let registry = client
        .display
        .get_registry(&mut client.conn.handle(), &client.event_queue.handle(), ())
        .unwrap();

    // Initial sync
    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let shm = client_ddata
        .globals
        .bind::<wayc::protocol::wl_shm::WlShm, _>(
            &mut client.conn.handle(),
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();

    let mut file = tempfile::tempfile().unwrap();
    write!(file, "I like trains!").unwrap();
    file.flush().unwrap();
    let pool = shm
        .create_pool(
            &mut client.conn.handle(),
            file.as_raw_fd(),
            42,
            &client.event_queue.handle(),
            (),
        )
        .unwrap();
    let buffer = pool
        .create_buffer(
            &mut client.conn.handle(),
            0,
            0,
            0,
            0,
            Format::Argb8888,
            &client.event_queue.handle(),
            (),
        )
        .unwrap();

    let compositor = client_ddata
        .globals
        .bind::<wayc::protocol::wl_compositor::WlCompositor, _>(
            &mut client.conn.handle(),
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();
    let surface = compositor
        .create_surface(&mut client.conn.handle(), &client.event_queue.handle(), ())
        .unwrap();
    surface.attach(&mut client.conn.handle(), Some(&buffer), 0, 0);

    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let surface_buffer = server_ddata.buffer_found.take().unwrap().unwrap();
    let (shm_fd, shm_buf) = server_ddata.fd_found.take().unwrap();
    let shm_buffer = shm_buf.unwrap();
    assert_eq!(surface_buffer, shm_buffer);

    let mut client_file = unsafe { File::from_raw_fd(shm_fd) };
    let mut contents = String::new();
    client_file.seek(SeekFrom::Start(0)).unwrap();
    client_file.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "I like trains!");
}

/*
 * Server Handler
 */

struct ServerHandler {
    buffer_found: Option<Option<ServerBuffer>>,
    fd_found: Option<(RawFd, Option<ServerBuffer>)>,
}

impl ways::Dispatch<ways::protocol::wl_compositor::WlCompositor> for ServerHandler {
    type UserData = ();
    fn request(
        &mut self,
        _: &ways::Client,
        _: &ways::protocol::wl_compositor::WlCompositor,
        request: ways::protocol::wl_compositor::Request,
        _: &(),
        _: &mut ways::DisplayHandle<'_, Self>,
        init: &mut ways::DataInit<'_, Self>,
    ) {
        if let ways::protocol::wl_compositor::Request::CreateSurface { id } = request {
            init.init(id, ());
        } else {
            panic!("Unexpected request!");
        }
    }
}

impl ways::Dispatch<ways::protocol::wl_surface::WlSurface> for ServerHandler {
    type UserData = ();
    fn request(
        &mut self,
        _: &ways::Client,
        _: &ways::protocol::wl_surface::WlSurface,
        request: ways::protocol::wl_surface::Request,
        _: &(),
        _: &mut ways::DisplayHandle<'_, Self>,
        _: &mut ways::DataInit<'_, Self>,
    ) {
        if let ways::protocol::wl_surface::Request::Attach { buffer, x, y } = request {
            assert_eq!(x, 0);
            assert_eq!(y, 0);
            assert!(self.buffer_found.is_none());
            self.buffer_found = Some(buffer);
        } else {
            panic!("Unexpected request!");
        }
    }
}

impl ways::Dispatch<ways::protocol::wl_shm::WlShm> for ServerHandler {
    type UserData = ();
    fn request(
        &mut self,
        _: &ways::Client,
        _: &ways::protocol::wl_shm::WlShm,
        request: ways::protocol::wl_shm::Request,
        _: &(),
        _: &mut ways::DisplayHandle<'_, Self>,
        init: &mut ways::DataInit<'_, Self>,
    ) {
        if let ways::protocol::wl_shm::Request::CreatePool { fd, size, id } = request {
            assert_eq!(size, 42);
            assert!(self.buffer_found.is_none());
            self.fd_found = Some((fd, None));
            init.init(id, ());
        } else {
            panic!("Unexpected request!");
        }
    }
}

impl ways::Dispatch<ways::protocol::wl_shm_pool::WlShmPool> for ServerHandler {
    type UserData = ();

    fn request(
        &mut self,
        _: &ways::Client,
        _: &ways::protocol::wl_shm_pool::WlShmPool,
        request: ways::protocol::wl_shm_pool::Request,
        _: &(),
        _: &mut ways::DisplayHandle<'_, Self>,
        init: &mut ways::DataInit<'_, Self>,
    ) {
        if let ways::protocol::wl_shm_pool::Request::CreateBuffer { id, .. } = request {
            let fd_found = self.fd_found.as_mut().unwrap();
            assert!(fd_found.1.is_none());
            fd_found.1 = Some(init.init(id, ()));
        }
    }
}

server_ignore_impl!(ServerHandler => [
    ways::protocol::wl_buffer::WlBuffer
]);

server_ignore_global_impl!(ServerHandler => [
    ways::protocol::wl_shm::WlShm,
    ways::protocol::wl_compositor::WlCompositor
]);

/*
 * Client Handler
 */
struct ClientHandler {
    globals: wayc::globals::GlobalList,
}

wayc::delegate_dispatch!(ClientHandler:
    [wayc::protocol::wl_registry::WlRegistry] => wayc::globals::GlobalList ; |me| { &mut me.globals }
);

client_ignore_impl!(ClientHandler => [
    wayc::protocol::wl_compositor::WlCompositor,
    wayc::protocol::wl_surface::WlSurface,
    wayc::protocol::wl_shm::WlShm,
    wayc::protocol::wl_shm_pool::WlShmPool,
    wayc::protocol::wl_buffer::WlBuffer
]);
