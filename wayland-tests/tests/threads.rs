use std::{sync::Barrier, thread};
use wayland_client::Proxy;
use wayland_tests::{TestServer, wayc};

#[test]
fn test_thread_destroy_object() {
    let mut server = TestServer::<()>::new();
    let (_, client) = server.add_client();

    let qh = client.event_queue.handle();
    let backend = client.conn.backend();

    for _ in 0..10 {
        let cb_id = client.display.sync(&qh, ()).id();

        let barrier = Barrier::new(2);
        thread::scope(|s| {
            s.spawn(|| {
                barrier.wait();
                for _ in 0..100 {
                    let _ = backend.get_data(cb_id.clone());
                }
            });

            barrier.wait();
            let _ = backend.destroy_object(&cb_id);
        });
    }
}

/*
// TODO consider why this panics without `client_system`,
// segfaults with `client_system`?
#[test]
fn test_thread_destroy_display() {
    let mut server = TestServer::<()>::new();

    for _ in 0..10 {
        let (_, client) = server.add_client::<()>();

        let backend = client.conn.backend();

        let display_id = client.display.id();

        let barrier = Barrier::new(2);
        thread::scope(|s| {
            s.spawn(|| {
                barrier.wait();
                for _ in 0..100 {
                    let _ = backend.get_data(display_id.clone());
                }
            });

            barrier.wait();
            let _ = backend.destroy_object(&display_id);
        });
    }
}
*/

#[test]
fn test_thread_destroys() {
    let mut server = TestServer::<()>::new();
    let (_, client) = server.add_client();

    let qh = client.event_queue.handle();
    let backend = client.conn.backend();

    for _ in 0..10000 {
        let cb_id = client.display.sync(&qh, ()).id();

        let barrier = Barrier::new(2);
        thread::scope(|s| {
            s.spawn(|| {
                barrier.wait();
                let _ = backend.destroy_object(&cb_id);
            });

            barrier.wait();
            let _ = backend.destroy_object(&cb_id);
        });
    }
}

struct ClientHandler;

impl wayc::Dispatch<wayc::protocol::wl_callback::WlCallback, ClientHandler> for () {
    fn event(
        &self,
        _: &mut ClientHandler,
        _: &wayc::protocol::wl_callback::WlCallback,
        _: wayc::protocol::wl_callback::Event,
        _: &wayc::Connection,
        _: &wayc::QueueHandle<ClientHandler>,
    ) {
    }
}
