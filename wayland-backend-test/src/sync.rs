use std::sync::atomic::{AtomicBool, Ordering};
use wayland_commons::client::ObjectData;

use crate::*;
struct SyncData(AtomicBool);

impl<B: ClientBackend> ObjectData<B> for SyncData {
    fn make_child(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn ObjectData<B>> {
        unimplemented!()
    }

    fn event(
        &self,
        _: &mut B::Handle,
        _: B::ObjectId,
        opcode: u16,
        args: &[Argument<B::ObjectId>],
    ) {
        assert_eq!(opcode, 0);
        assert!(matches!(args, [Argument::Uint(_)]));
        self.0.store(true, Ordering::SeqCst);
    }

    fn destroyed(&self, _: B::ObjectId) {}
}

// send a wl_display.sync request and receive the response
fn test<C: ClientBackend, S: ServerBackend + ServerPolling<S>>() {
    let mut test = TestPair::<C, S>::init();

    // send the request
    let client_display = test.client.handle().display_id();
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::WL_CALLBACK_INTERFACE, 1)));
    let sync_data = Arc::new(SyncData(AtomicBool::new(false)));
    let sync_id = test
        .client
        .handle()
        .send_constructor(
            client_display,
            0,
            &[Argument::NewId(placeholder)],
            Some(sync_data.clone()),
        )
        .unwrap();
    test.client_flush().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    // process it server-side
    test.server_dispatch().unwrap();
    test.server_flush().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    // ensure the answer is received client-side
    test.client_dispatch().unwrap();
    assert!(sync_data.0.load(Ordering::SeqCst));
    // and the sync object should be dead
    assert!(test.client.handle().get_data(sync_id).is_err());
}

expand_test!(test);

fn test_bad_placeholder<C: ClientBackend, S: ServerBackend + ServerPolling<S>>() {
    let mut test = TestPair::<C, S>::init();

    // send the request
    let client_display = test.client.handle().display_id();
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::WL_REGISTRY_INTERFACE, 1)));
    let sync_data = Arc::new(SyncData(AtomicBool::new(false)));
    let sync_id = test
        .client
        .handle()
        .send_constructor(
            client_display,
            0,
            &[Argument::NewId(placeholder)],
            Some(sync_data.clone()),
        )
        .unwrap();
    test.client_flush().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    // process it server-side
    test.server_dispatch().unwrap();
    test.server_flush().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    // ensure the answer is received client-side
    test.client_dispatch().unwrap();
    assert!(sync_data.0.load(Ordering::SeqCst));
    // and the sync object should be dead
    assert!(test.client.handle().get_data(sync_id).is_err());
}

expand_test!(panic test_bad_placeholder);

fn test_bad_signature<C: ClientBackend, S: ServerBackend + ServerPolling<S>>() {
    let mut test = TestPair::<C, S>::init();

    // send the request
    let client_display = test.client.handle().display_id();
    let sync_data = Arc::new(SyncData(AtomicBool::new(false)));
    let sync_id = test
        .client
        .handle()
        .send_constructor(client_display, 0, &[Argument::Uint(1)], Some(sync_data.clone()))
        .unwrap();
    test.client_flush().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    // process it server-side
    test.server_dispatch().unwrap();
    test.server_flush().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    // ensure the answer is received client-side
    test.client_dispatch().unwrap();
    assert!(sync_data.0.load(Ordering::SeqCst));
    // and the sync object should be dead
    assert!(test.client.handle().get_data(sync_id).is_err());
}

expand_test!(panic test_bad_signature);
