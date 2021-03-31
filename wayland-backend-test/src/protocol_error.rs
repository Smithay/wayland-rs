use std::{
    ffi::CString,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use wayland_commons::client::ObjectData;

use crate::*;

struct ServerData<S: ServerBackend<()>>(Arc<Mutex<Option<S::ObjectId>>>);

impl<S: ServerBackend<()> + 'static> GlobalHandler<(), S> for ServerData<S> {
    fn make_data(self: Arc<Self>, _: &mut (), _: &ObjectInfo) -> Arc<dyn ServerObjectData<(), S>> {
        Arc::new(DoNothingData)
    }

    fn bind(
        &self,
        _: &mut S::Handle,
        _: &mut (),
        _: S::ClientId,
        _: S::GlobalId,
        object_id: S::ObjectId,
    ) {
        *(self.0.lock().unwrap()) = Some(object_id);
    }
}

fn test<C: ClientBackend + 'static, S: ServerBackend<()> + ServerPolling<(), S> + 'static>() {
    let mut test = TestPair::<(), C, S>::init();

    let object_id = Arc::new(Mutex::new(None));

    // Prepare a global
    test.server.handle().create_global(
        &interfaces::TEST_GLOBAL_INTERFACE,
        3,
        Arc::new(ServerData(object_id.clone())),
    );

    // get the registry client-side
    let client_display = test.client.handle().display_id();
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::WL_REGISTRY_INTERFACE, 1)));
    let registry_id = test
        .client
        .handle()
        .send_request(
            client_display,
            1,
            &[Argument::NewId(placeholder)],
            Some(Arc::new(DoNothingData)),
        )
        .unwrap();
    // create the test global
    let placeholder =
        test.client.handle().placeholder_id(Some((&interfaces::TEST_GLOBAL_INTERFACE, 3)));
    test.client
        .handle()
        .send_request(
            registry_id,
            0,
            &[
                Argument::Uint(1),
                Argument::Str(Box::new(
                    CString::new(interfaces::TEST_GLOBAL_INTERFACE.name.as_bytes()).unwrap(),
                )),
                Argument::Uint(3),
                Argument::NewId(placeholder),
            ],
            Some(Arc::new(DoNothingData)),
        )
        .unwrap();

    test.client_flush().unwrap();
    test.server_dispatch(&mut ()).unwrap();

    // get the object_id for the global
    let oid = object_id.lock().unwrap().clone().unwrap();

    // post the error
    test.server.handle().post_error(oid, 42, CString::new("I don't like you.".as_bytes()).unwrap());

    test.server_flush().unwrap();
    let ret = test.client_dispatch();

    match ret {
        Err(WaylandError::Protocol(err)) => {
            assert_eq!(err.code, 42);
            assert_eq!(err.object_id, 3);
            assert_eq!(err.object_interface, "test_global");
            if std::any::TypeId::of::<C>() == std::any::TypeId::of::<client_rs>() {
                // only the RS client backed can retrieve the error message
                assert_eq!(err.message, "I don't like you.");
            }
        }
        _ => panic!("Bad ret: {:?}", ret),
    }
}

expand_test!(test);
