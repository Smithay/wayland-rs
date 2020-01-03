extern crate tempfile;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::sync::{Arc, Mutex};

mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use wayc::protocol::wl_shm::Format;

use ways::protocol::wl_buffer::WlBuffer as ServerBuffer;

fn insert_compositor(server: &mut TestServer) -> Arc<Mutex<Option<Option<ServerBuffer>>>> {
    use ways::protocol::{wl_compositor, wl_surface};

    let buffer_found = Arc::new(Mutex::new(None));
    let buffer_found2 = buffer_found.clone();

    ways::request_enum!(Reqs |
        Compositor => wl_compositor::WlCompositor,
        Surface => wl_surface::WlSurface
    );

    let filter = ways::Filter::new(move |req, filter, _| match req {
        Reqs::Compositor {
            request: wl_compositor::Request::CreateSurface { id: surface },
            ..
        } => {
            surface.assign(filter.clone());
        }
        Reqs::Surface {
            request: wl_surface::Request::Attach { buffer, x, y },
            ..
        } => {
            assert!(x == 0);
            assert!(y == 0);
            assert!(buffer_found.lock().unwrap().is_none());
            *(buffer_found.lock().unwrap()) = Some(buffer);
        }
        _ => panic!("Unexpected request."),
    });

    server
        .display
        .create_global::<wl_compositor::WlCompositor, _>(1, move |compositor, version, _| {
            assert!(version == 1);
            compositor.assign(filter.clone());
        });

    buffer_found2
}

fn insert_shm(server: &mut TestServer) -> Arc<Mutex<Option<(RawFd, Option<ways::Main<ServerBuffer>>)>>> {
    use ways::protocol::{wl_shm, wl_shm_pool};

    let buffer = Arc::new(Mutex::new(None));
    let buffer2 = buffer.clone();

    ways::request_enum!(Reqs |
        Shm => wl_shm::WlShm,
        Pool => wl_shm_pool::WlShmPool
    );

    let filter = ways::Filter::new(move |req, filter, _| match req {
        Reqs::Shm {
            request: wl_shm::Request::CreatePool { id, fd, size },
            ..
        } => {
            assert!(size == 42);
            assert!(buffer.lock().unwrap().is_none());
            *buffer.lock().unwrap() = Some((fd, None));
            id.assign(filter.clone());
        }
        Reqs::Pool {
            request: wl_shm_pool::Request::CreateBuffer { id, .. },
            ..
        } => {
            let mut guard = buffer.lock().unwrap();
            let buf = guard.as_mut().unwrap();
            assert!(buf.1.is_none());
            id.quick_assign(|_, _, _| {});
            buf.1 = Some(id);
        }
        _ => {
            panic!("Unexpected request");
        }
    });

    server
        .display
        .create_global::<wl_shm::WlShm, _>(1, move |shm, version, _| {
            assert!(version == 1);
            shm.assign(filter.clone());
        });

    buffer2
}

#[test]
fn attach_null() {
    // Server setup
    //
    let mut server = TestServer::new();
    let buffer_found = insert_compositor(&mut server);

    // Client setup
    //
    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    // Initial sync
    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_exact::<wayc::protocol::wl_compositor::WlCompositor>(1)
        .unwrap();
    let surface = compositor.create_surface();
    surface.attach(None, 0, 0);

    roundtrip(&mut client, &mut server).unwrap();

    assert!(*buffer_found.lock().unwrap() == Some(None));
}

#[test]
fn attach_buffer() {
    // Server setup
    //
    let mut server = TestServer::new();
    let buffer_found = insert_compositor(&mut server);
    let fd_found = insert_shm(&mut server);

    // Client setup
    //
    let mut client = TestClient::new(&server.socket_name);
    let manager = wayc::GlobalManager::new(&client.display_proxy);

    // Initial sync
    roundtrip(&mut client, &mut server).unwrap();

    let shm = manager
        .instantiate_exact::<wayc::protocol::wl_shm::WlShm>(1)
        .unwrap();

    let mut file = tempfile::tempfile().unwrap();
    write!(file, "I like trains!").unwrap();
    file.flush().unwrap();
    let pool = shm.create_pool(file.as_raw_fd(), 42);
    let buffer = pool.create_buffer(0, 0, 0, 0, Format::Argb8888);

    let compositor = manager
        .instantiate_exact::<wayc::protocol::wl_compositor::WlCompositor>(1)
        .unwrap();
    let surface = compositor.create_surface();
    surface.attach(Some(&buffer), 0, 0);

    roundtrip(&mut client, &mut server).unwrap();

    let surface_buffer = buffer_found.lock().unwrap().take().unwrap().unwrap();
    let (shm_fd, shm_buf) = fd_found.lock().unwrap().take().unwrap();
    let shm_buffer = shm_buf.unwrap();
    assert!(&surface_buffer == &*shm_buffer);

    let mut client_file = unsafe { File::from_raw_fd(shm_fd) };
    let mut contents = String::new();
    client_file.seek(SeekFrom::Start(0)).unwrap();
    client_file.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "I like trains!");
}
