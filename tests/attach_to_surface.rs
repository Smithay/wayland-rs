extern crate tempfile;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::sync::{Arc, Mutex};

mod helpers;

use helpers::{roundtrip, wayc, ways, TestClient, TestServer};

use wayc::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use wayc::protocol::wl_shm::{Format, RequestsTrait as ShmRequests};
use wayc::protocol::wl_shm_pool::RequestsTrait as PoolRequests;
use wayc::protocol::wl_surface::RequestsTrait as SurfaceRequests;

use ways::protocol::wl_buffer::WlBuffer as ServerBuffer;
use ways::Resource;

fn insert_compositor(server: &mut TestServer) -> Arc<Mutex<Option<Option<Resource<ServerBuffer>>>>> {
    use ways::protocol::{wl_compositor, wl_surface};

    let buffer_found = Arc::new(Mutex::new(None));
    let buffer_found2 = buffer_found.clone();

    let loop_token = server.event_loop.token();
    server.display.create_global::<wl_compositor::WlCompositor, _>(
        &loop_token,
        1,
        move |compositor, version| {
            assert!(version == 1);
            let compositor_buffer_found = buffer_found.clone();
            compositor.implement(
                move |event, _| {
                    if let wl_compositor::Request::CreateSurface { id: surface } = event {
                        let my_buffer_found = compositor_buffer_found.clone();
                        surface.implement(
                            move |event, _| {
                                if let wl_surface::Request::Attach { buffer, x, y } = event {
                                    assert!(x == 0);
                                    assert!(y == 0);
                                    assert!(my_buffer_found.lock().unwrap().is_none());
                                    *(my_buffer_found.lock().unwrap()) = Some(buffer);
                                } else {
                                    panic!("Unexpected request on surface!");
                                }
                            },
                            None::<fn(_)>,
                            (),
                        );
                    } else {
                        panic!("Unexpected request on compositor!");
                    }
                },
                None::<fn(_)>,
                (),
            );
        },
    );

    buffer_found2
}

fn insert_shm(server: &mut TestServer) -> Arc<Mutex<Option<(RawFd, Option<Resource<ServerBuffer>>)>>> {
    use ways::protocol::{wl_shm, wl_shm_pool};

    let buffer = Arc::new(Mutex::new(None));
    let buffer2 = buffer.clone();

    let loop_token = server.event_loop.token();
    server
        .display
        .create_global::<wl_shm::WlShm, _>(&loop_token, 1, move |shm, version| {
            assert!(version == 1);
            let shm_buffer = buffer.clone();
            shm.implement(
                move |req, _| {
                    let wl_shm::Request::CreatePool { id, fd, size } = req;
                    assert!(size == 42);
                    assert!(shm_buffer.lock().unwrap().is_none());
                    *shm_buffer.lock().unwrap() = Some((fd, None));
                    let pool_buffer = shm_buffer.clone();
                    id.implement(
                        move |req, _| {
                            if let wl_shm_pool::Request::CreateBuffer { id, .. } = req {
                                let mut buffer_guard = pool_buffer.lock().unwrap();
                                let buf = buffer_guard.as_mut().unwrap();
                                assert!(buf.1.is_none());
                                buf.1 = Some(id.implement(|_, _| {}, None::<fn(_)>, ()));
                            } else {
                                panic!("Unexpected request on buffer!");
                            }
                        },
                        None::<fn(_)>,
                        (),
                    );
                },
                None::<fn(_)>,
                (),
            );
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
    let manager = wayc::GlobalManager::new(&client.display);

    // Initial sync
    roundtrip(&mut client, &mut server).unwrap();

    let compositor = manager
        .instantiate_exact::<wayc::protocol::wl_compositor::WlCompositor, _>(1, |comp| {
            comp.implement(|_, _| {}, ())
        })
        .unwrap();
    let surface = compositor
        .create_surface(|surface| surface.implement(|_, _| {}, ()))
        .unwrap();
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
    let manager = wayc::GlobalManager::new(&client.display);

    // Initial sync
    roundtrip(&mut client, &mut server).unwrap();

    let shm = manager
        .instantiate_exact::<wayc::protocol::wl_shm::WlShm, _>(1, |shm| shm.implement(|_, _| {}, ()))
        .unwrap();

    let mut file = tempfile::tempfile().unwrap();
    write!(file, "I like trains!").unwrap();
    file.flush().unwrap();
    let pool = shm
        .create_pool(file.as_raw_fd(), 42, |newp| newp.implement(|_, _| {}, ()))
        .unwrap();
    let buffer = pool
        .create_buffer(0, 0, 0, 0, Format::Argb8888, |newb| newb.implement(|_, _| {}, ()))
        .unwrap();

    let compositor = manager
        .instantiate_exact::<wayc::protocol::wl_compositor::WlCompositor, _>(1, |comp| {
            comp.implement(|_, _| {}, ())
        })
        .unwrap();
    let surface = compositor
        .create_surface(|surface| surface.implement(|_, _| {}, ()))
        .unwrap();
    surface.attach(Some(&buffer), 0, 0);

    roundtrip(&mut client, &mut server).unwrap();

    let surface_buffer = buffer_found.lock().unwrap().take().unwrap().unwrap();
    let (shm_fd, shm_buf) = fd_found.lock().unwrap().take().unwrap();
    let shm_buffer = shm_buf.unwrap();
    assert!(surface_buffer == shm_buffer);

    let mut client_file = unsafe { File::from_raw_fd(shm_fd) };
    let mut contents = String::new();
    client_file.seek(SeekFrom::Start(0)).unwrap();
    client_file.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "I like trains!");
}
