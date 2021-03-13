use std::{
    ffi::CString,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::*;

struct ServerData(AtomicBool);

impl<S: ServerBackend<()>> ServerObjectData<(), S> for ServerData {
    fn make_child(self: Arc<Self>, _: &mut (), _: &ObjectInfo) -> Arc<dyn ServerObjectData<(), S>> {
        self
    }

    fn request(
        &self,
        handle: &mut S::Handle,
        _: &mut (),
        _: S::ClientId,
        object: S::ObjectId,
        opcode: u16,
        arguments: &[Argument<S::ObjectId>],
    ) {
        if opcode == 1 {
            assert_eq!(handle.object_info(object.clone()).unwrap().interface.name, "test_global");
            if let [Argument::NewId(secondary)] = arguments {
                handle.send_event(object, 1, &[Argument::Object(secondary.clone())]).unwrap();
            } else {
                panic!("Bad argument list!");
            }
        } else if opcode == 3 {
            assert_eq!(handle.object_info(object).unwrap().interface.name, "test_global");
            if let [Argument::Object(secondary), Argument::Object(tertiary), Argument::Uint(u)] =
                arguments
            {
                assert_eq!(
                    handle.object_info(secondary.clone()).unwrap().interface.name,
                    "secondary"
                );
                if *u == 1 {
                    assert!(tertiary.is_null());
                } else if *u == 2 {
                    assert_eq!(
                        handle.object_info(tertiary.clone()).unwrap().interface.name,
                        "tertiary"
                    );
                    self.0.store(true, Ordering::SeqCst);
                }
            } else {
                panic!("Bad argument list!");
            }
        }
    }

    fn destroyed(&self, _: S::ClientId, _: S::ObjectId) {}
}

impl<S: ServerBackend<()>> GlobalHandler<(), S> for ServerData {
    fn make_data(self: Arc<Self>, _: &mut (), _: &ObjectInfo) -> Arc<dyn ServerObjectData<(), S>> {
        self
    }

    fn bind(&self, _: &mut S::Handle, _: &mut (), _: S::ClientId, _: S::GlobalId, _: S::ObjectId) {}
}

struct ClientData(AtomicBool);

impl<C: ClientBackend> ClientObjectData<C> for ClientData {
    fn make_child(self: Arc<Self>, child_info: &ObjectInfo) -> Arc<dyn ClientObjectData<C>> {
        self
    }
    fn event(
        &self,
        handle: &mut C::Handle,
        object_id: C::ObjectId,
        opcode: u16,
        arguments: &[Argument<C::ObjectId>],
    ) {
        assert_eq!(opcode, 1);
        if let [Argument::Object(secondary)] = arguments {
            let info = handle.info(secondary.clone()).unwrap();
            assert_eq!(info.id, 4);
            assert_eq!(info.interface.name, "secondary");
        } else {
            panic!("Bad argument list!");
        }
        self.0.store(true, Ordering::SeqCst);
    }
    fn destroyed(&self, object_id: C::ObjectId) {}
}

// create a global and create objects
fn test<C: ClientBackend, S: ServerBackend<()> + ServerPolling<(), S>>() {
    let mut test = TestPair::<(), C, S>::init();

    let server_data = Arc::new(ServerData(AtomicBool::new(false)));
    let client_data = Arc::new(ClientData(AtomicBool::new(false)));

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
            Some(client_data.clone()),
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
    let null_obj = test.client.handle().null_id();
    test.client
        .handle()
        .send_request(
            test_global_id.clone(),
            3,
            &[
                Argument::Object(secondary_id.clone()),
                Argument::Object(null_obj),
                Argument::Uint(1),
            ],
        )
        .unwrap();
    test.client
        .handle()
        .send_request(
            test_global_id.clone(),
            3,
            &[Argument::Object(secondary_id), Argument::Object(tertiary_id), Argument::Uint(2)],
        )
        .unwrap();

    test.client_flush().unwrap();
    test.server_dispatch(&mut ()).unwrap();
    test.server_flush().unwrap();
    test.client_dispatch().unwrap();

    assert!(server_data.0.load(Ordering::SeqCst));
    assert!(client_data.0.load(Ordering::SeqCst));
}

expand_test!(test);

fn test_bad_interface<C: ClientBackend, S: ServerBackend<()> + ServerPolling<(), S>>() {
    let mut test = TestPair::<(), C, S>::init();

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

fn test_double_null<C: ClientBackend, S: ServerBackend<()> + ServerPolling<(), S>>() {
    let mut test = TestPair::<(), C, S>::init();

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
    let null_obj = test.client.handle().null_id();
    // link them, argument order is wrong, should panic
    test.client
        .handle()
        .send_request(
            test_global_id.clone(),
            3,
            &[Argument::Object(null_obj.clone()), Argument::Object(null_obj), Argument::Uint(42)],
        )
        .unwrap();
}
expand_test!(panic test_double_null);
