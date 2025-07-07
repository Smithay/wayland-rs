extern crate tempfile;

use std::io::ErrorKind;
use std::os::unix::io::{AsFd, OwnedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use wayland_backend::client::ObjectData;

#[macro_use]
mod helpers;

use helpers::{globals, wayc, ways, TestClient, TestServer};

// This test checks the behavior for
//  - sending many wl_shm.create_pool requests between connection flushes
//    that results in more file descriptors being queued for sending
//    than the wayland_backend::rs::socket::MAX_FDS_OUT limit
//  - sending many wl_display.sync requests between connection flushes
//    that results in more message bytes being queued for sending
//    than the wayland_backend::rs::socket::MAX_BYTES_OUT limit
//
// This test is based on tests/attach_to_surface.rs
// and uses a modified version of the roundtrip function from tests/helpers/mod.rs

// each wl_shm.create_pool will send 1 file descriptor
// TEST_FD_COUNT > wayland_backend::rs::socket::MAX_FDS_OUT = 28
const TEST_FD_COUNT: usize = 60;

// each wl_display.sync will send a header and a new_id totaling 12 bytes
// TEST_SYNC_COUNT * 12 > wayland_backend::rs::socket::MAX_BYTES_OUT = 4096
const TEST_SYNC_COUNT: usize = 500;

#[test]
fn backend_socket_out_limits() {
    // Server setup
    let mut server = TestServer::new();
    server
        .display
        .handle()
        .create_global::<ServerHandler, ways::protocol::wl_compositor::WlCompositor, _>(1, ());
    server.display.handle().create_global::<ServerHandler, ways::protocol::wl_shm::WlShm, _>(1, ());
    let mut server_ddata = ServerHandler { received_fds: Vec::new() };

    // Client setup
    let (_, mut client) = server.add_client();
    let mut client_ddata = ClientHandler { globals: globals::GlobalList::new(), syncs_done: 0 };

    let registry = client.display.get_registry(&client.event_queue.handle(), ());

    // Initial sync
    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();

    let shm = client_ddata
        .globals
        .bind::<wayc::protocol::wl_shm::WlShm, _, _>(
            &client.event_queue.handle(),
            &registry,
            1..2,
            (),
        )
        .unwrap();

    // Queue more fds than wayland_backend::rs::socket::MAX_FDS_OUT
    let mut files = Vec::new();
    let mut pools = Vec::new();
    for _ in 0..TEST_FD_COUNT {
        let file = tempfile::tempfile().unwrap();
        let pool = shm.create_pool(file.as_fd(), 8, &client.event_queue.handle(), ());
        files.push(file);
        pools.push(pool);
    }
    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();
    assert_eq!(server_ddata.received_fds.len(), TEST_FD_COUNT);

    // Queue more bytes than wayland_backend::rs::socket::MAX_BYTES_OUT
    for _ in 0..TEST_SYNC_COUNT {
        client.display.sync(&client.event_queue.handle(), ());
    }
    roundtrip(&mut client, &mut server, &mut client_ddata, &mut server_ddata).unwrap();
    assert_eq!(client_ddata.syncs_done, TEST_SYNC_COUNT);
}

struct ServerHandler {
    received_fds: Vec<OwnedFd>,
}

impl ways::Dispatch<ways::protocol::wl_shm::WlShm, ()> for ServerHandler {
    fn request(
        state: &mut Self,
        _: &ways::Client,
        _: &ways::protocol::wl_shm::WlShm,
        request: ways::protocol::wl_shm::Request,
        _: &(),
        _: &ways::DisplayHandle,
        init: &mut ways::DataInit<'_, Self>,
    ) {
        if let ways::protocol::wl_shm::Request::CreatePool { fd, id, .. } = request {
            state.received_fds.push(fd);
            init.init(id, ());
        } else {
            panic!("Unexpected request!");
        }
    }
}

server_ignore_impl!(ServerHandler => [
    ways::protocol::wl_compositor::WlCompositor,
    ways::protocol::wl_shm_pool::WlShmPool
]);

server_ignore_global_impl!(ServerHandler => [
    ways::protocol::wl_compositor::WlCompositor,
    ways::protocol::wl_shm::WlShm
]);

struct ClientHandler {
    globals: globals::GlobalList,
    syncs_done: usize,
}

impl AsMut<globals::GlobalList> for ClientHandler {
    fn as_mut(&mut self) -> &mut globals::GlobalList {
        &mut self.globals
    }
}

impl wayc::Dispatch<wayc::protocol::wl_callback::WlCallback, ()> for ClientHandler {
    fn event(
        state: &mut Self,
        _: &wayc::protocol::wl_callback::WlCallback,
        _: <wayc::protocol::wl_callback::WlCallback as wayland_client::Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        state.syncs_done += 1;
    }
}

wayc::delegate_dispatch!(ClientHandler:
    [wayc::protocol::wl_registry::WlRegistry: ()] => globals::GlobalList
);

client_ignore_impl!(ClientHandler => [
    wayc::protocol::wl_compositor::WlCompositor,
    wayc::protocol::wl_shm::WlShm,
    wayc::protocol::wl_shm_pool::WlShmPool
]);

// Use a modified version of helpers::roundtrip here
// which gracefully handles ErrorKind::WouldBlock from socket reads and flushes
// to enable testing with the large amount of Wayland messages this test requires
fn roundtrip<CD: 'static, SD: 'static>(
    client: &mut TestClient<CD>,
    server: &mut TestServer<SD>,
    client_ddata: &mut CD,
    server_ddata: &mut SD,
) -> Result<(), wayc::backend::WaylandError> {
    // send to the server
    let done = Arc::new(AtomicBool::new(false));
    let done2 = done.clone();
    client
        .conn
        .send_request(
            &client.display,
            wayc::protocol::wl_display::Request::Sync {},
            Some(Arc::new(SyncData { done })),
        )
        .unwrap();
    // Proper WouldBlock handling would need polling like in real applications.
    // So just ignore WouldBlock up to 1000 times,
    // that is enough for this test if the loop makes progress,
    // and avoids the busy loop if the test is failing with no progress.
    let mut would_block_count_left = 1000;
    while !done2.load(Ordering::Acquire) {
        match client.conn.flush() {
            Ok(_) => {}
            Err(wayc::backend::WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => {
                would_block_count_left -= 1;
                if would_block_count_left == 0 {
                    return Err(wayc::backend::WaylandError::Io(e));
                }
            }
            Err(e) => return Err(e),
        }
        // Also inline here a modified version of helpers::TestServer::answer
        // which handles ErrorKind::WouldBlock
        match server.display.dispatch_clients(server_ddata) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                would_block_count_left -= 1;
                if would_block_count_left == 0 {
                    return Err(wayc::backend::WaylandError::Io(e));
                }
            }
            Err(e) => panic!("{e:?}"),
        };
        match server.display.flush_clients() {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                would_block_count_left -= 1;
                if would_block_count_left == 0 {
                    return Err(wayc::backend::WaylandError::Io(e));
                }
            }
            Err(e) => panic!("{e:?}"),
        }
        // dispatch all client-side
        client.event_queue.dispatch_pending(client_ddata).unwrap();
        let e = client.conn.prepare_read().map(|guard| guard.read()).unwrap_or(Ok(0));
        // even if read_events returns an error, some messages may need dispatching
        client.event_queue.dispatch_pending(client_ddata).unwrap();
        match e {
            Ok(_) => {}
            Err(wayc::backend::WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => {
                would_block_count_left -= 1;
                if would_block_count_left == 0 {
                    return Err(wayc::backend::WaylandError::Io(e));
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

struct SyncData {
    done: Arc<AtomicBool>,
}

impl wayc::backend::ObjectData for SyncData {
    fn event(
        self: Arc<Self>,
        _backend: &wayc::backend::Backend,
        _msg: self::wayc::backend::protocol::Message<wayc::backend::ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn ObjectData>> {
        self.done.store(true, Ordering::Release);
        None
    }

    fn destroyed(&self, _: wayc::backend::ObjectId) {}
}
