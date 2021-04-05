use std::{
    ffi::CString,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::protocol::Message;

use super::*;

struct ServerData(AtomicBool);

macro_rules! impl_server_objectdata {
    ($server_backend:tt) => {
        impl $server_backend::ObjectData<()> for ServerData {
            fn make_child(self: Arc<Self>, _: &mut (), _: &ObjectInfo) -> Arc<dyn $server_backend::ObjectData<()>> {
                self
            }

            fn request(
                &self,
                handle: &mut $server_backend::Handle<()>,
                _: &mut (),
                _: $server_backend::ClientId,
                msg: Message<$server_backend::ObjectId>,
            ) {
                if msg.opcode == 1 {
                    assert_eq!(
                        handle.object_info(msg.sender_id.clone()).unwrap().interface.name,
                        "test_global"
                    );
                    if let [Argument::NewId(secondary)] = &msg.args[..] {
                        handle
                            .send_event(message!(msg.sender_id, 1, [Argument::Object(secondary.clone())]))
                            .unwrap();
                    } else {
                        panic!("Bad argument list!");
                    }
                } else if msg.opcode == 3 {
                    assert_eq!(handle.object_info(msg.sender_id).unwrap().interface.name, "test_global");
                    if let [Argument::Object(secondary), Argument::Object(tertiary), Argument::Uint(u)] =
                        &msg.args[..]
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

            fn destroyed(&self, _: $server_backend::ClientId, _: $server_backend::ObjectId) {}
        }

        impl $server_backend::GlobalHandler<()> for ServerData {
            fn make_data(self: Arc<Self>, _: &mut (), _: &ObjectInfo) -> Arc<dyn $server_backend::ObjectData<()>> {
                self
            }

            fn bind(&self, _: &mut $server_backend::Handle<()>, _: &mut (), _: $server_backend::ClientId, _: $server_backend::GlobalId, _: $server_backend::ObjectId) {}
        }
    }
}

impl_server_objectdata!(server_rs);
impl_server_objectdata!(server_sys);

struct ClientData(AtomicBool);

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
                assert_eq!(msg.opcode, 1);
                if let [Argument::Object(secondary)] = &msg.args[..] {
                    let info = handle.info(secondary.clone()).unwrap();
                    assert_eq!(info.id, 4);
                    assert_eq!(info.interface.name, "secondary");
                } else {
                    panic!("Bad argument list!");
                }
                self.0.store(true, Ordering::SeqCst);
            }
            fn destroyed(&self, _object_id: $client_backend::ObjectId) {}
        }
    };
}

impl_client_objectdata!(client_rs);
impl_client_objectdata!(client_sys);

// create a global and create objects
expand_test!(create_objects, {
    let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut server = server_backend::Backend::new().unwrap();
    let _client_id = server.insert_client(rx, Arc::new(DoNothingData)).unwrap();
    let mut client = client_backend::Backend::connect(tx).unwrap();

    let server_data = Arc::new(ServerData(AtomicBool::new(false)));
    let client_data = Arc::new(ClientData(AtomicBool::new(false)));

    // Prepare a global
    server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 3, server_data.clone());

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
    let test_global_id = client
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
            Some(client_data.clone()),
        )
        .unwrap();
    // create the two objects
    let placeholder = client.handle().placeholder_id(None);
    let secondary_id = client
        .handle()
        .send_request(message!(test_global_id.clone(), 1, [Argument::NewId(placeholder)]), None)
        .unwrap();
    let placeholder = client.handle().placeholder_id(None);
    let tertiary_id = client
        .handle()
        .send_request(message!(test_global_id.clone(), 2, [Argument::NewId(placeholder)]), None)
        .unwrap();
    // link them
    let null_obj = client.handle().null_id();
    client
        .handle()
        .send_request(
            message!(
                test_global_id.clone(),
                3,
                [
                    Argument::Object(secondary_id.clone()),
                    Argument::Object(null_obj),
                    Argument::Uint(1),
                ],
            ),
            None,
        )
        .unwrap();
    client
        .handle()
        .send_request(
            message!(
                test_global_id.clone(),
                3,
                [Argument::Object(secondary_id), Argument::Object(tertiary_id), Argument::Uint(2)],
            ),
            None,
        )
        .unwrap();

    client.flush().unwrap();
    server.dispatch_events(&mut ()).unwrap();
    server.flush(None).unwrap();
    client.dispatch_events().unwrap();

    assert!(server_data.0.load(Ordering::SeqCst));
    assert!(client_data.0.load(Ordering::SeqCst));
});

expand_test!(panic bad_interface, {
    let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut server = server_backend::Backend::new().unwrap();
    let _client_id = server.insert_client(rx, Arc::new(DoNothingData)).unwrap();
    let mut client = client_backend::Backend::connect(tx).unwrap();

    let server_data = Arc::new(ServerData(AtomicBool::new(false)));

    // Prepare a global
    server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 3, server_data.clone());

    // get the registry client-side
    let client_display = client.handle().display_id();
    let placeholder =
        client.handle().placeholder_id(Some((&interfaces::WL_REGISTRY_INTERFACE, 1)));
    let registry_id = client
        .handle()
        .send_request(
            message!(client_display, 1, [Argument::NewId(placeholder)],),
            Some(Arc::new(DoNothingData)),
        )
        .unwrap();
    // create the test global
    let placeholder = client.handle().placeholder_id(Some((&interfaces::TEST_GLOBAL_INTERFACE, 3)));
    let test_global_id = client
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
            None,
        )
        .unwrap();
    // create the two objects
    let placeholder = client.handle().placeholder_id(None);
    let secondary_id = client
        .handle()
        .send_request(message!(test_global_id.clone(), 1, [Argument::NewId(placeholder)]), None)
        .unwrap();
    let placeholder = client.handle().placeholder_id(None);
    let tertiary_id = client
        .handle()
        .send_request(message!(test_global_id.clone(), 2, [Argument::NewId(placeholder)]), None)
        .unwrap();
    // link them, argument order is wrong, should panic
    client
        .handle()
        .send_request(
            message!(
                test_global_id.clone(),
                3,
                [Argument::Object(tertiary_id), Argument::Object(secondary_id), Argument::Uint(42)],
            ),
            None,
        )
        .unwrap();
});

expand_test!(panic double_null, {
    let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut server = server_backend::Backend::new().unwrap();
    let _client_id = server.insert_client(rx, Arc::new(DoNothingData)).unwrap();
    let mut client = client_backend::Backend::connect(tx).unwrap();

    let server_data = Arc::new(ServerData(AtomicBool::new(false)));

    // Prepare a global
    server.handle().create_global(&interfaces::TEST_GLOBAL_INTERFACE, 3, server_data.clone());

    // get the registry client-side
    let client_display = client.handle().display_id();
    let placeholder =
        client.handle().placeholder_id(Some((&interfaces::WL_REGISTRY_INTERFACE, 1)));
    let registry_id = client
        .handle()
        .send_request(
            message!(client_display, 1, [Argument::NewId(placeholder)],),
            Some(Arc::new(DoNothingData)),
        )
        .unwrap();
    // create the test global
    let placeholder =
        client.handle().placeholder_id(Some((&interfaces::TEST_GLOBAL_INTERFACE, 3)));
    let test_global_id = client
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
            None,
        )
        .unwrap();
    // create the two objects
    let null_obj = client.handle().null_id();
    // link them, first object cannot be null, shoudl panic
    client
        .handle()
        .send_request(
            message!(
                test_global_id.clone(),
                3,
                [
                    Argument::Object(null_obj.clone()),
                    Argument::Object(null_obj),
                    Argument::Uint(42)
                ],
            ),
            None,
        )
        .unwrap();
});
