use std::{
    ffi::CString,
    sync::atomic::{AtomicU32, Ordering},
};

use wayland_commons::{client::ObjectId, message, Message};

use crate::*;

struct ServerData;

impl<S: ServerBackend<()>> ServerObjectData<(), S> for ServerData {
    fn make_child(self: Arc<Self>, _: &mut (), _: &ObjectInfo) -> Arc<dyn ServerObjectData<(), S>> {
        self
    }

    fn request(
        &self,
        handle: &mut S::Handle,
        _: &mut (),
        _: S::ClientId,
        _msg: Message<S::ObjectId>,
    ) {
    }

    fn destroyed(&self, _: S::ClientId, _: S::ObjectId) {}
}

impl<S: ServerBackend<()>> GlobalHandler<(), S> for ServerData {
    fn make_data(self: Arc<Self>, _: &mut (), _: &ObjectInfo) -> Arc<dyn ServerObjectData<(), S>> {
        self
    }

    fn bind(
        &self,
        handle: &mut S::Handle,
        _: &mut (),
        client: S::ClientId,
        _: S::GlobalId,
        object_id: S::ObjectId,
    ) {
        // send the first event with a newid & a null object
        let obj_1 = handle
            .create_object(client.clone(), &interfaces::QUAD_INTERFACE, 3, Arc::new(ServerData))
            .unwrap();
        let null_id = handle.null_id();
        handle
            .send_event(message!(
                object_id.clone(),
                2,
                [Argument::NewId(obj_1.clone()), Argument::Object(null_id)],
            ))
            .unwrap();
        // send the second
        let obj_2 = handle
            .create_object(client, &interfaces::QUAD_INTERFACE, 3, Arc::new(ServerData))
            .unwrap();
        handle
            .send_event(message!(
                object_id.clone(),
                2,
                [Argument::NewId(obj_2), Argument::Object(obj_1)]
            ))
            .unwrap();
    }
}

struct ClientData(AtomicU32);

impl<C: ClientBackend> ClientObjectData<C> for ClientData {
    fn make_child(self: Arc<Self>, _child_info: &ObjectInfo) -> Arc<dyn ClientObjectData<C>> {
        self
    }
    fn event(&self, handle: &mut C::Handle, msg: Message<C::ObjectId>) {
        assert_eq!(msg.opcode, 2);
        if self.0.load(Ordering::SeqCst) == 0 {
            if let [Argument::NewId(obj_1), Argument::Object(null_id)] = &msg.args[..] {
                let info = handle.info(obj_1.clone()).unwrap();
                assert_eq!(info.id, 0xFF00_0000);
                assert_eq!(info.interface.name, "quad");
                assert!(null_id.is_null());
            } else {
                panic!("Bad argument list!");
            }
            self.0.store(1, Ordering::SeqCst);
        } else {
            if let [Argument::NewId(obj_2), Argument::Object(obj_1)] = &msg.args[..] {
                // check obj1
                let info = handle.info(obj_1.clone()).unwrap();
                assert_eq!(info.id, 0xFF00_0000);
                assert_eq!(info.interface.name, "quad");
                // check obj2
                let info = handle.info(obj_2.clone()).unwrap();
                assert_eq!(info.id, 0xFF00_0001);
                assert_eq!(info.interface.name, "quad");
            } else {
                panic!("Bad argument list!");
            }
            self.0.store(2, Ordering::SeqCst);
        }
    }
    fn destroyed(&self, _object_id: C::ObjectId) {}
}

fn test<C: ClientBackend, S: ServerBackend<()> + ServerPolling<(), S>>() {
    let mut test = TestPair::<(), C, S>::init();

    let client_data = Arc::new(ClientData(AtomicU32::new(0)));

    // Prepare a global
    test.server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 3, Arc::new(ServerData));

    // get the registry client-side
    let client_display = test.client.handle().display_id();
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::WL_REGISTRY_INTERFACE, 1)));
    let registry_id = test
        .client
        .handle()
        .send_request(
            message!(client_display, 1, [Argument::NewId(placeholder)],),
            Some(Arc::new(DoNothingData)),
        )
        .unwrap();
    // create the test global
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::TEST_GLOBAL_INTERFACE, 3)));
    let test_global_id = test
        .client
        .handle()
        .send_request(
            message!(
                registry_id,
                0,
                [
                    Argument::Uint(1),
                    Argument::Str(Box::new(
                        CString::new(interfaces::TEST_GLOBAL_INTERFACE.name.as_bytes()).unwrap(),
                    )),
                    Argument::Uint(1),
                    Argument::NewId(placeholder),
                ],
            ),
            Some(client_data.clone()),
        )
        .unwrap();

    test.client_flush().unwrap();
    test.server_dispatch(&mut ()).unwrap();
    test.server_flush().unwrap();
    test.client_dispatch().unwrap();

    assert_eq!(client_data.0.load(Ordering::SeqCst), 2);
}

expand_test!(test);
