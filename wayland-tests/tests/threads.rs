use std::{
    os::fd::OwnedFd,
    sync::{Arc, Barrier},
    thread,
};
use wayland_client::Proxy;
use wayland_tests::{TestServer, wayc};

#[test]
fn test_thread_destroy_object() {
    let mut server = TestServer::<()>::new();
    let (_, client) = server.add_client::<()>();

    let qh = client.event_queue.handle();
    let backend = client.conn.backend();

    for _ in 0..10 {
        let cb_id = client.display.sync(&qh, wayc::NoopIgnore).id();

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
                    // get_data on the display should succeed
                    let _ = backend.get_data(display_id.clone());
                }
            });

            barrier.wait();
            // destroy_object on the display should return InvalidId
            assert!(backend.destroy_object(&display_id).is_err());
        });
    }
}

#[test]
fn test_thread_destroys() {
    let mut server = TestServer::<()>::new();
    let (_, client) = server.add_client::<()>();

    let qh = client.event_queue.handle();
    let backend = client.conn.backend();

    for _ in 0..10000 {
        let cb_id = client.display.sync(&qh, wayc::NoopIgnore).id();

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

// Minimal test for `set_data`
// TODO Test racing `set_data` calls
#[test]
fn test_set_data() {
    let mut server = TestServer::<()>::new();
    let (_, client) = server.add_client::<()>();

    let qh = client.event_queue.handle();
    let backend = client.conn.backend();

    let cb_id = client.display.sync(&qh, wayc::NoopIgnore).id();

    backend
        .get_data(cb_id.clone())
        .unwrap()
        .data_as_any()
        .downcast_ref::<wayc::NoopIgnore>()
        .unwrap();
    backend
        .get_data(cb_id.clone())
        .unwrap()
        .data_as_any()
        .downcast_ref::<wayc::NoopIgnore>()
        .unwrap();
    backend.set_data(cb_id.clone(), Arc::new(CustomObjectData)).unwrap();
    let data = backend.get_data(cb_id.clone()).unwrap();
    let data = data.data_as_any();
    assert!(data.downcast_ref::<wayc::NoopIgnore>().is_none());
    data.downcast_ref::<CustomObjectData>().unwrap();
}

struct CustomObjectData;

impl wayc::backend::ObjectData for CustomObjectData {
    fn event(
        self: Arc<Self>,
        _backend: &wayc::backend::Backend,
        _msg: wayc::backend::protocol::Message<wayc::backend::ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn wayc::backend::ObjectData>> {
        None
    }

    fn destroyed(&self, _object_id: wayc::backend::ObjectId) {}
}
