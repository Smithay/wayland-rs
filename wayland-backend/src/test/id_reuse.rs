//! Regression for Smithay/wayland-rs#949 (server/sys):
//! When a client destroys an object and creates another in the same flush,
//! `ObjectData::destroyed` must run only after the whole dispatch batch so
//! mid-batch id reuse cannot UAF `ResourceUserData`.
//!
//! Only `server_sys` is asserted: the pure-Rust server has a separate code path
//! and #949 is specifically the libwayland (`use_system_lib`) backend.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use super::*;

struct Shared {
    creates: AtomicUsize,
    destroys: AtomicUsize,
    /// Set when `get_secondary` runs during a dispatch.
    saw_create_in_dispatch: AtomicBool,
    /// Captured in `destroyed`: whether a create already ran in this batch.
    destroyed_after_reuse_create: AtomicBool,
}

impl server_sys::ObjectData<()> for Shared {
    fn request(
        self: Arc<Self>,
        _: &server_sys::Handle,
        _: &mut (),
        _: &server_sys::ClientId,
        msg: OwnedMessage<server_sys::ObjectId>,
    ) -> Option<Arc<dyn server_sys::ObjectData<()>>> {
        // test_global.get_secondary
        if msg.opcode == 1 {
            self.creates.fetch_add(1, Ordering::SeqCst);
            self.saw_create_in_dispatch.store(true, Ordering::SeqCst);
            return Some(self);
        }
        None
    }

    fn destroyed(
        self: Arc<Self>,
        _: &server_sys::Handle,
        _: &mut (),
        _: &server_sys::ClientId,
        _: &server_sys::ObjectId,
    ) {
        self.destroys.fetch_add(1, Ordering::SeqCst);
        self.destroyed_after_reuse_create
            .store(self.saw_create_in_dispatch.load(Ordering::SeqCst), Ordering::SeqCst);
    }
}

impl server_sys::GlobalHandler<()> for Shared {
    fn bind(
        self: Arc<Self>,
        _: &server_sys::Handle,
        _: &mut (),
        _: &server_sys::ClientId,
        _: &server_sys::GlobalId,
        _: &server_sys::ObjectId,
    ) -> Arc<dyn server_sys::ObjectData<()>> {
        self
    }
}

macro_rules! id_reuse_sys_test {
    ($test_name:ident, $client_backend:ident, $body:block) => {
        #[test]
        fn $test_name() {
            let _ = env_logger::builder().is_test(true).try_init();
            use $client_backend as client_backend;
            use server_sys as server_backend;
            $body
        }
    };
}

id_reuse_sys_test!(
    destroy_then_create_same_batch_defers_destroyed__client_sys__server_sys,
    client_sys,
    {
        let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();
        let server = server_backend::Backend::new().unwrap();
        let _client_id = server.handle().insert_client(rx, Arc::new(())).unwrap();
        let client = client_backend::Backend::connect(tx).unwrap();

        let shared = Arc::new(Shared {
            creates: AtomicUsize::new(0),
            destroys: AtomicUsize::new(0),
            saw_create_in_dispatch: AtomicBool::new(false),
            destroyed_after_reuse_create: AtomicBool::new(false),
        });

        server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 5, shared.clone());

        let client_display = client.display_id();
        let registry_id = client
            .send_request(
                message!(&client_display, 1, [Argument::NewId(client_backend::ObjectId::null())],),
                Some(Arc::new(DoNothingData)),
                Some((&interfaces::WL_REGISTRY_INTERFACE, 1)),
            )
            .unwrap();
        let test_global_id = client
            .send_request(
                message!(
                    &registry_id,
                    0,
                    [
                        Argument::Uint(1),
                        Argument::Str(Some(Box::new(interfaces::TEST_GLOBAL_INTERFACE.name))),
                        Argument::Uint(5),
                        Argument::NewId(client_backend::ObjectId::null()),
                    ],
                ),
                Some(Arc::new(DoNothingData)),
                Some((&interfaces::TEST_GLOBAL_INTERFACE, 5)),
            )
            .unwrap();

        let secondary_a = client
            .send_request(
                message!(&test_global_id, 1, [Argument::NewId(client_backend::ObjectId::null())]),
                Some(Arc::new(DoNothingData)),
                None,
            )
            .unwrap();

        client.flush().unwrap();
        server.dispatch_all_clients(&mut ()).unwrap();
        server.flush(None).unwrap();
        let _ = client.prepare_read().unwrap().read();

        assert_eq!(shared.creates.load(Ordering::SeqCst), 1);

        shared.saw_create_in_dispatch.store(false, Ordering::SeqCst);
        shared.destroyed_after_reuse_create.store(false, Ordering::SeqCst);

        // One flush: destroy secondary, then create another. One server dispatch.
        client.send_request(message!(&secondary_a, 0, []), None, None).unwrap();
        let secondary_b = client
            .send_request(
                message!(&test_global_id, 1, [Argument::NewId(client_backend::ObjectId::null())]),
                Some(Arc::new(DoNothingData)),
                None,
            )
            .unwrap();

        client.flush().unwrap();
        server.dispatch_all_clients(&mut ()).unwrap();
        server.flush(None).unwrap();
        let _ = client.prepare_read().unwrap().read();

        assert_eq!(shared.creates.load(Ordering::SeqCst), 2);
        assert_eq!(shared.destroys.load(Ordering::SeqCst), 1);
        assert!(
            shared.destroyed_after_reuse_create.load(Ordering::SeqCst),
            "ObjectData::destroyed ran before the same-batch create; \
             ResourceUserData must not be torn down mid wl_event_loop_dispatch (#949)"
        );

        client.send_request(message!(&secondary_b, 0, []), None, None).unwrap();
        client.flush().unwrap();
        server.dispatch_all_clients(&mut ()).unwrap();
        assert_eq!(shared.destroys.load(Ordering::SeqCst), 2);
    }
);

id_reuse_sys_test!(
    destroy_then_create_same_batch_defers_destroyed__client_rs__server_sys,
    client_rs,
    {
        let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();
        let server = server_backend::Backend::new().unwrap();
        let _client_id = server.handle().insert_client(rx, Arc::new(())).unwrap();
        let client = client_backend::Backend::connect(tx).unwrap();

        let shared = Arc::new(Shared {
            creates: AtomicUsize::new(0),
            destroys: AtomicUsize::new(0),
            saw_create_in_dispatch: AtomicBool::new(false),
            destroyed_after_reuse_create: AtomicBool::new(false),
        });

        server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 5, shared.clone());

        let client_display = client.display_id();
        let registry_id = client
            .send_request(
                message!(&client_display, 1, [Argument::NewId(client_backend::ObjectId::null())],),
                Some(Arc::new(DoNothingData)),
                Some((&interfaces::WL_REGISTRY_INTERFACE, 1)),
            )
            .unwrap();
        let test_global_id = client
            .send_request(
                message!(
                    &registry_id,
                    0,
                    [
                        Argument::Uint(1),
                        Argument::Str(Some(Box::new(interfaces::TEST_GLOBAL_INTERFACE.name))),
                        Argument::Uint(5),
                        Argument::NewId(client_backend::ObjectId::null()),
                    ],
                ),
                Some(Arc::new(DoNothingData)),
                Some((&interfaces::TEST_GLOBAL_INTERFACE, 5)),
            )
            .unwrap();

        let secondary_a = client
            .send_request(
                message!(&test_global_id, 1, [Argument::NewId(client_backend::ObjectId::null())]),
                Some(Arc::new(DoNothingData)),
                None,
            )
            .unwrap();

        client.flush().unwrap();
        server.dispatch_all_clients(&mut ()).unwrap();
        server.flush(None).unwrap();
        let _ = client.prepare_read().unwrap().read();

        assert_eq!(shared.creates.load(Ordering::SeqCst), 1);

        shared.saw_create_in_dispatch.store(false, Ordering::SeqCst);
        shared.destroyed_after_reuse_create.store(false, Ordering::SeqCst);

        client.send_request(message!(&secondary_a, 0, []), None, None).unwrap();
        let secondary_b = client
            .send_request(
                message!(&test_global_id, 1, [Argument::NewId(client_backend::ObjectId::null())]),
                Some(Arc::new(DoNothingData)),
                None,
            )
            .unwrap();

        client.flush().unwrap();
        server.dispatch_all_clients(&mut ()).unwrap();
        server.flush(None).unwrap();
        let _ = client.prepare_read().unwrap().read();

        assert_eq!(shared.creates.load(Ordering::SeqCst), 2);
        assert_eq!(shared.destroys.load(Ordering::SeqCst), 1);
        assert!(
            shared.destroyed_after_reuse_create.load(Ordering::SeqCst),
            "ObjectData::destroyed ran before the same-batch create; \
             ResourceUserData must not be torn down mid wl_event_loop_dispatch (#949)"
        );

        client.send_request(message!(&secondary_b, 0, []), None, None).unwrap();
        client.flush().unwrap();
        server.dispatch_all_clients(&mut ()).unwrap();
        assert_eq!(shared.destroys.load(Ordering::SeqCst), 2);
    }
);
