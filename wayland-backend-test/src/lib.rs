#![allow(dead_code)]

use std::{
    os::unix::{net::UnixStream, prelude::AsRawFd},
    sync::Arc,
};

use wayland_commons::{
    client::{BackendHandle as ClientHandle, ClientBackend},
    core_interfaces::*,
    server::{
        BackendHandle as ServerHandle, ClientData, CommonPollBackend, DisconnectReason,
        IndependentBackend, ServerBackend,
    },
    Argument, ObjectInfo,
};

use wayland_backend_rs::{
    client::Backend as client_rs, server::IndependentServerBackend as server_independent_rs,
};

macro_rules! expand_test {
    ($test_name:ident) => {
        expand_test!(__expand, $test_name, client_rs, server_independent_rs);
    };
    (__expand, $test_name: ident, $client_backend: ty, $server_backend: ty) => {
        concat_idents::concat_idents!(fn_name = $test_name, __, $client_backend, __, $server_backend {
            #[test]
            fn fn_name() {
                $test_name::<$client_backend,$server_backend>();
            }
        });
    };
}

struct DoNothingClientData;

impl<S: ServerBackend> ClientData<S> for DoNothingClientData {
    fn initialized(&self, _client_id: S::ClientId) {}
    fn disconnected(&self, _client_id: S::ClientId, _reason: DisconnectReason) {}
}

struct TestPair<C: ClientBackend, S: ServerBackend> {
    pub client: C,
    pub server: S,
    pub client_id: <S as ServerBackend>::ClientId,
}

impl<C: ClientBackend, S: ServerBackend> TestPair<C, S> {
    fn init() -> TestPair<C, S> {
        TestPair::init_with_data(Arc::new(DoNothingClientData))
    }

    fn init_with_data(data: Arc<dyn ClientData<S>>) -> TestPair<C, S> {
        let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();

        let mut server = S::new().unwrap();
        let client_id = unsafe { server.insert_client(rx, data) };

        let client = unsafe { C::connect(tx) }.unwrap();

        TestPair { client, server, client_id }
    }
}

// send a wl_display.sync request and receive the response
fn independent_sync<C: ClientBackend, S: ServerBackend + IndependentBackend>() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use wayland_commons::client::ObjectData;
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

    let mut test = TestPair::<C, S>::init();

    // send the request
    let client_display = test.client.handle().display_id();
    let placeholder = test.client.handle().placeholder_id(Some((&WL_CALLBACK_INTERFACE, 1)));
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
    test.client.flush().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    // process it server-side
    test.server.dispatch_events_for(test.client_id.clone()).unwrap();
    test.server.flush(Some(test.client_id)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    // ensure the answer is received client-side
    test.client.dispatch_events().unwrap();
    assert!(sync_data.0.load(Ordering::SeqCst));
    // and the sync object should be dead
    assert!(test.client.handle().get_data(sync_id).is_err());
}

expand_test!(independent_sync);
