use std::{
    ffi::{CStr, CString},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::*;

struct ServerData(AtomicBool);

impl<S: ServerBackend> ServerObjectData<S> for ServerData {
    fn make_child(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn ServerObjectData<S>> {
        self
    }

    fn request(
        &self,
        handle: &mut S::Handle,
        _: S::ClientId,
        object: S::ObjectId,
        opcode: u16,
        arguments: &[Argument<S::ObjectId>],
    ) {
        if opcode == 3 {
            assert_eq!(handle.object_info(object).unwrap().interface.name, "test_global");
            if let [Argument::Object(secondary), Argument::Object(tertiary), Argument::Uint(u)] =
                arguments
            {
                assert_eq!(
                    handle.object_info(secondary.clone()).unwrap().interface.name,
                    "secondary"
                );
                assert_eq!(
                    handle.object_info(tertiary.clone()).unwrap().interface.name,
                    "tertiary"
                );
                assert_eq!(*u, 42);
                self.0.store(true, Ordering::SeqCst);
            } else {
                panic!("Bad argument list!");
            }
        }
    }

    fn destroyed(&self, _: S::ClientId, _: S::ObjectId) {}
}

impl<S: ServerBackend> GlobalHandler<S> for ServerData {
    fn make_data(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn ServerObjectData<S>> {
        self
    }

    fn bind(&self, _: S::ClientId, _: S::GlobalId, _: S::ObjectId) {}
}

// create a global and send the many_args method
fn test<C: ClientBackend, S: ServerBackend + ServerPolling<S>>() {
    let mut test = TestPair::<C, S>::init();

    let server_data = Arc::new(ServerData(AtomicBool::new(false)));

    // Prepare a global
    test.server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 3, server_data.clone());

    // get the registry client-side
    let client_display = test.client.handle().display_id();
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::WL_REGISTRY_INTERFACE, 1)));
    let registry_id = test
        .client
        .handle()
        .send_constructor(
            client_display,
            1,
            &[Argument::NewId(placeholder)],
            Some(Arc::new(DoNothingData)),
        )
        .unwrap();
    // create the test global
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::TEST_GLOBAL_INTERFACE, 3)));
    let test_global_id = test
        .client
        .handle()
        .send_constructor(
            registry_id,
            0,
            &[
                Argument::Uint(1),
                Argument::Str(Box::new(
                    CString::new(interfaces::TEST_GLOBAL_INTERFACE.name.as_bytes()).unwrap(),
                )),
                Argument::Uint(1),
                Argument::NewId(placeholder),
            ],
            None,
        )
        .unwrap();
    // create the two objects
    let placeholder = test.client.handle().placeholder_id(None);
    let secondary_id = test
        .client
        .handle()
        .send_constructor(test_global_id.clone(), 1, &[Argument::NewId(placeholder)], None)
        .unwrap();
    let placeholder = test.client.handle().placeholder_id(None);
    let tertiary_id = test
        .client
        .handle()
        .send_constructor(test_global_id.clone(), 2, &[Argument::NewId(placeholder)], None)
        .unwrap();
    // link them
    test.client
        .handle()
        .send_request(
            test_global_id.clone(),
            3,
            &[Argument::Object(secondary_id), Argument::Object(tertiary_id), Argument::Uint(42)],
        )
        .unwrap();

    test.client_flush().unwrap();

    test.server_dispatch().unwrap();

    assert!(server_data.0.load(Ordering::SeqCst));
}

expand_test!(test);

fn test_bad_interface<C: ClientBackend, S: ServerBackend + ServerPolling<S>>() {
    let mut test = TestPair::<C, S>::init();

    let server_data = Arc::new(ServerData(AtomicBool::new(false)));

    // Prepare a global
    test.server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 3, server_data.clone());

    // get the registry client-side
    let client_display = test.client.handle().display_id();
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::WL_REGISTRY_INTERFACE, 1)));
    let registry_id = test
        .client
        .handle()
        .send_constructor(
            client_display,
            1,
            &[Argument::NewId(placeholder)],
            Some(Arc::new(DoNothingData)),
        )
        .unwrap();
    // create the test global
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::TEST_GLOBAL_INTERFACE, 3)));
    let test_global_id = test
        .client
        .handle()
        .send_constructor(
            registry_id,
            0,
            &[
                Argument::Uint(1),
                Argument::Str(Box::new(
                    CString::new(interfaces::TEST_GLOBAL_INTERFACE.name.as_bytes()).unwrap(),
                )),
                Argument::Uint(1),
                Argument::NewId(placeholder),
            ],
            None,
        )
        .unwrap();
    // create the two objects
    let placeholder = test.client.handle().placeholder_id(None);
    let secondary_id = test
        .client
        .handle()
        .send_constructor(test_global_id.clone(), 1, &[Argument::NewId(placeholder)], None)
        .unwrap();
    let placeholder = test.client.handle().placeholder_id(None);
    let tertiary_id = test
        .client
        .handle()
        .send_constructor(test_global_id.clone(), 2, &[Argument::NewId(placeholder)], None)
        .unwrap();
    // link them, argument order is wrong, should panic
    test.client
        .handle()
        .send_request(
            test_global_id.clone(),
            3,
            &[Argument::Object(tertiary_id), Argument::Object(secondary_id), Argument::Uint(42)],
        )
        .unwrap();
}
expand_test!(panic test_bad_interface);
