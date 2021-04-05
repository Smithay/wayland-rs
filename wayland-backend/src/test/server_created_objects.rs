use std::{
    ffi::CString,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::protocol::Message;

use super::*;

struct ServerData;

macro_rules! impl_globalhandler {
    ($server_backend:tt) => {
        impl $server_backend::GlobalHandler<()> for ServerData {
            fn make_data(
                self: Arc<Self>,
                _: &mut (),
                _: &ObjectInfo,
            ) -> Arc<dyn $server_backend::ObjectData<()>> {
                Arc::new(DoNothingData)
            }

            fn bind(
                &self,
                handle: &mut $server_backend::Handle<()>,
                _: &mut (),
                client: $server_backend::ClientId,
                _: $server_backend::GlobalId,
                object_id: $server_backend::ObjectId,
            ) {
                // send the first event with a newid & a null object
                let obj_1 = handle
                    .create_object(
                        client.clone(),
                        &interfaces::QUAD_INTERFACE,
                        3,
                        Arc::new(DoNothingData),
                    )
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
                    .create_object(client, &interfaces::QUAD_INTERFACE, 3, Arc::new(DoNothingData))
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
    };
}

impl_globalhandler!(server_rs);
impl_globalhandler!(server_sys);

struct ClientData(AtomicU32);

macro_rules! impl_client_objectdata {
    ($client_backend:tt) => {
        impl $client_backend::ObjectData for ClientData {
            fn make_child(
                self: Arc<Self>,
                _child_info: &ObjectInfo,
            ) -> Arc<dyn $client_backend::ObjectData> {
                self
            }
            fn event(
                &self,
                handle: &mut $client_backend::Handle,
                msg: Message<$client_backend::ObjectId>,
            ) {
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
            fn destroyed(&self, _object_id: $client_backend::ObjectId) {}
        }
    };
}

impl_client_objectdata!(client_rs);
impl_client_objectdata!(client_sys);

expand_test!(server_created_object, {
    let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut server = server_backend::Backend::new().unwrap();
    let _client_id = server.insert_client(rx, Arc::new(DoNothingData)).unwrap();
    let mut client = client_backend::Backend::connect(tx).unwrap();

    let client_data = Arc::new(ClientData(AtomicU32::new(0)));

    // Prepare a global
    server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 3, Arc::new(ServerData));

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
    let _test_global_id = client
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

    client.flush().unwrap();
    server.dispatch_events(&mut ()).unwrap();
    server.flush(None).unwrap();
    client.dispatch_events().unwrap();

    assert_eq!(client_data.0.load(Ordering::SeqCst), 2);
});
