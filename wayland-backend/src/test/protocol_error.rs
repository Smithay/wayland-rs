use std::{
    ffi::CString,
    sync::{Arc, Mutex},
};

use super::*;

struct ServerData<Id>(Arc<Mutex<Option<Id>>>);

impl server_rs::GlobalHandler<()> for ServerData<server_rs::ObjectId> {
    fn make_data(
        self: Arc<Self>,
        _: &mut (),
        _: &ObjectInfo,
    ) -> Arc<dyn server_rs::ObjectData<()>> {
        Arc::new(DoNothingData)
    }

    fn bind(
        &self,
        _: &mut server_rs::Handle<()>,
        _: &mut (),
        _: server_rs::ClientId,
        _: server_rs::GlobalId,
        object_id: server_rs::ObjectId,
    ) {
        *(self.0.lock().unwrap()) = Some(object_id);
    }
}

impl server_sys::GlobalHandler<()> for ServerData<server_sys::ObjectId> {
    fn make_data(
        self: Arc<Self>,
        _: &mut (),
        _: &ObjectInfo,
    ) -> Arc<dyn server_sys::ObjectData<()>> {
        Arc::new(DoNothingData)
    }

    fn bind(
        &self,
        _: &mut server_sys::Handle<()>,
        _: &mut (),
        _: server_sys::ClientId,
        _: server_sys::GlobalId,
        object_id: server_sys::ObjectId,
    ) {
        *(self.0.lock().unwrap()) = Some(object_id);
    }
}

expand_test!(protocol_error, {
    let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut server = server_backend::Backend::new().unwrap();
    let _client_id = server.insert_client(rx, Arc::new(DoNothingData)).unwrap();
    let mut client = client_backend::Backend::connect(tx).unwrap();

    let object_id = Arc::new(Mutex::new(None));

    // Prepare a global
    server.handle().create_global(
        &interfaces::TEST_GLOBAL_INTERFACE,
        3,
        Arc::new(ServerData(object_id.clone())),
    );

    // get the registry client-side
    let client_display = client.handle().display_id();
    let placeholder = client.handle().placeholder_id(Some((&interfaces::WL_REGISTRY_INTERFACE, 1)));
    let registry_id = client
        .handle()
        .send_request(
            message!(client_display, 1, [Argument::NewId(placeholder)],),
            Some(Arc::new(DoNothingData)),
        )
        .unwrap();
    // create the test global
    let placeholder = client.handle().placeholder_id(Some((&interfaces::TEST_GLOBAL_INTERFACE, 3)));
    client
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
                    Argument::Uint(3),
                    Argument::NewId(placeholder),
                ],
            ),
            Some(Arc::new(DoNothingData)),
        )
        .unwrap();

    client.flush().unwrap();
    server.dispatch_events(&mut ()).unwrap();

    // get the object_id for the global
    let oid = object_id.lock().unwrap().clone().unwrap();

    // post the error
    server.handle().post_error(oid, 42, CString::new("I don't like you.".as_bytes()).unwrap());

    server.flush(None).unwrap();
    let ret = client.dispatch_events();

    match ret {
        Err(client_backend::WaylandError::Protocol(err)) => {
            assert_eq!(err.code, 42);
            assert_eq!(err.object_id, 3);
            assert_eq!(err.object_interface, "test_global");
            if std::any::TypeId::of::<client_backend::Backend>()
                == std::any::TypeId::of::<client_rs::Backend>()
            {
                // only the RS client backed can retrieve the error message
                assert_eq!(err.message, "I don't like you.");
            }
        }
        _ => panic!("Bad ret: {:?}", ret),
    }
});
