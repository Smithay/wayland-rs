use std::{
    ffi::CString,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use super::*;

struct ServerData(AtomicBool);

macro_rules! impl_server_objectdata {
    ($server_backend:tt) => {
        impl $server_backend::ObjectData<()> for ServerData {
            fn request(
                self: Arc<Self>,
                _: &mut $server_backend::Handle<()>,
                _: &mut (),
                _: $server_backend::ClientId,
                _: Message<$server_backend::ObjectId>,
            ) -> Option<Arc<dyn $server_backend::ObjectData<()>>> {
                None
            }

            fn destroyed(&self, _: $server_backend::ClientId, _: $server_backend::ObjectId) {
                self.0.store(true, Ordering::Release);
            }
        }

        impl $server_backend::GlobalHandler<()> for ServerData {
            fn bind(
                self: Arc<Self>,
                _: &mut $server_backend::Handle<()>,
                _: &mut (),
                _: $server_backend::ClientId,
                _: $server_backend::GlobalId,
                _: $server_backend::ObjectId,
            ) -> Arc<dyn $server_backend::ObjectData<()>> {
                self
            }
        }
    };
}

impl_server_objectdata!(server_rs);
impl_server_objectdata!(server_sys);

struct ClientData(AtomicBool);

macro_rules! impl_client_objectdata {
    ($client_backend:tt) => {
        impl $client_backend::ObjectData for ClientData {
            fn event(
                self: Arc<Self>,
                _: &mut $client_backend::Handle,
                _: Message<$client_backend::ObjectId>,
            ) -> Option<Arc<dyn $client_backend::ObjectData>> {
                None
            }
            fn destroyed(&self, _object_id: $client_backend::ObjectId) {
                self.0.store(true, Ordering::Release);
            }
        }
    };
}

impl_client_objectdata!(client_rs);
impl_client_objectdata!(client_sys);

expand_test!(destructor_request, {
    let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut server = server_backend::Backend::new().unwrap();
    let _client_id = server.insert_client(rx, Arc::new(DoNothingData)).unwrap();
    let mut client = client_backend::Backend::connect(tx).unwrap();

    let server_data = Arc::new(ServerData(AtomicBool::new(false)));
    let client_data = Arc::new(ClientData(AtomicBool::new(false)));

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

    client
        .handle()
        .send_request(
            message!(
                test_global_id,
                4, // destroy
                []
            ),
            None,
        )
        .unwrap();

    assert!(client_data.0.load(Ordering::Acquire));

    client.flush().unwrap();

    server.dispatch_all_clients(&mut ()).unwrap();

    assert!(server_data.0.load(Ordering::Acquire));
});

expand_test!(destructor_cleanup, {
    let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut server = server_backend::Backend::new().unwrap();
    let _client_id = server.insert_client(rx, Arc::new(DoNothingData)).unwrap();
    let mut client = client_backend::Backend::connect(tx).unwrap();

    let server_data = Arc::new(ServerData(AtomicBool::new(false)));
    let client_data = Arc::new(ClientData(AtomicBool::new(false)));

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
            Some(client_data),
        )
        .unwrap();

    client.flush().unwrap();

    // dispatch once to ensure objects are created server-side as well
    server.dispatch_all_clients(&mut ()).unwrap();

    // then drop the client
    std::mem::drop(client);

    // now destructors should be called
    server.dispatch_all_clients(&mut ()).unwrap();
    server.flush(None).unwrap();

    assert!(server_data.0.load(Ordering::Acquire));
});

struct ServerClientData(AtomicBool);

macro_rules! impl_server_clientdata {
    ($server_backend:tt) => {
        impl $server_backend::ClientData<()> for ServerClientData {
            fn initialized(&self, _: $server_backend::ClientId) {}

            fn disconnected(
                &self,
                _: $server_backend::ClientId,
                _: crate::types::server::DisconnectReason,
            ) {
                self.0.store(true, Ordering::Release);
            }
        }
    };
}
impl_server_clientdata!(server_rs);
impl_server_clientdata!(server_sys);

expand_test!(destructor_client_cleanup, {
    let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut server = server_backend::Backend::new().unwrap();
    let client_data = Arc::new(ServerClientData(AtomicBool::new(false)));
    let _client_id = server.insert_client(rx, client_data.clone()).unwrap();

    std::mem::drop(tx);

    server.dispatch_all_clients(&mut ()).unwrap();

    assert!(client_data.0.load(Ordering::Acquire));
});
