#![allow(dead_code, non_snake_case)]

use std::sync::Arc;

use wayland_commons::{
    client::{
        BackendHandle as ClientHandle, ClientBackend, ObjectData as ClientObjectData, WaylandError,
    },
    server::{
        BackendHandle as ServerHandle, ClientData, CommonPollBackend, DisconnectReason,
        GlobalHandler, IndependentBackend, ObjectData as ServerObjectData,
        ObjectId as ServerObjectId, ServerBackend,
    },
    Argument, Message, ObjectInfo,
};

use wayland_backend_rs::{
    client::Backend as client_rs, server::CommonPollServerBackend as server_common_rs,
    server::IndependentServerBackend as server_independent_rs,
};

use wayland_backend_sys::{client::Backend as client_sys, server::Backend as server_sys};

macro_rules! expand_test {
    ($test_name:ident) => {
        expand_test!(__list_expand, __no_panic, $test_name, ());
    };
    (panic $test_name:ident) => {
        expand_test!(__list_expand, __panic, $test_name, ());
    };
    (__list_expand, $panic:tt, $test_name:ident, $data:ty) => {
        expand_test!(__expand, $panic, $test_name, client_rs, server_independent_rs, $data);
        expand_test!(__expand, $panic, $test_name, client_rs, server_common_rs, $data);
        expand_test!(__expand, $panic, $test_name, client_sys, server_independent_rs, $data);
        expand_test!(__expand, $panic, $test_name, client_rs, server_sys, $data);
        expand_test!(__expand, $panic, $test_name, client_sys, server_sys, $data);
    };
    (__expand, __panic, $test_name: ident, $client_backend: ty, $server_backend: ident, $data: ty) => {
        concat_idents::concat_idents!(fn_name = $test_name, __, $client_backend, __, $server_backend {
            #[test]
            #[should_panic]
            fn fn_name() {
                let _ = env_logger::builder().is_test(true).try_init();
                $test_name::<$client_backend,$server_backend<$data>>();
            }
        });
    };
    (__expand, __no_panic, $test_name: ident, $client_backend: ty, $server_backend: ident, $data: ty) => {
        concat_idents::concat_idents!(fn_name = $test_name, __, $client_backend, __, $server_backend {
            #[test]
            fn fn_name() {
                let _ = env_logger::builder().is_test(true).try_init();
                $test_name::<$client_backend,$server_backend<$data>>();
            }
        });
    };
}

mod interfaces {
    wayland_scanner::generate_interfaces!("../tests/scanner_assets/test-protocol.xml");
}

impl<D, C: ClientBackend, S: ServerBackend<D> + ServerPolling<D, S>> TestPair<D, C, S> {
    fn init() -> TestPair<D, C, S> {
        TestPair::init_with_data(Arc::new(DoNothingData))
    }

    fn init_with_data(data: Arc<dyn ClientData<D, S>>) -> TestPair<D, C, S> {
        let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();

        let mut server = S::new().unwrap();
        let client_id = server.insert_client(rx, data).unwrap();
        let client = C::connect(tx).unwrap();

        TestPair { client, server, client_id }
    }

    fn client_dispatch(&mut self) -> Result<usize, WaylandError> {
        self.client.dispatch_events()
    }

    fn client_flush(&mut self) -> Result<(), WaylandError> {
        self.client.flush()
    }

    fn server_dispatch(&mut self, data: &mut D) -> std::io::Result<usize> {
        self.server.poll_client(data, self.client_id.clone())
    }

    fn server_flush(&mut self) -> std::io::Result<()> {
        self.server.flush(Some(self.client_id.clone()))
    }
}

// A generic trait for polling any kind of server in tests
trait ServerPolling<D, S: ServerBackend<D>> {
    fn poll_client(&mut self, data: &mut D, id: S::ClientId) -> std::io::Result<usize>;
}

// We need to do manual implementations, because otherwise we get conflicting implementations

impl<D> ServerPolling<D, server_independent_rs<D>> for server_independent_rs<D> {
    fn poll_client(
        &mut self,
        data: &mut D,
        id: <server_independent_rs<D> as ServerBackend<D>>::ClientId,
    ) -> std::io::Result<usize> {
        self.dispatch_events_for(data, id)
    }
}

impl<D> ServerPolling<D, server_common_rs<D>> for server_common_rs<D> {
    fn poll_client(
        &mut self,
        data: &mut D,
        _: <server_independent_rs<D> as ServerBackend<D>>::ClientId,
    ) -> std::io::Result<usize> {
        self.dispatch_events(data)
    }
}

impl<D> ServerPolling<D, server_sys<D>> for server_sys<D> {
    fn poll_client(
        &mut self,
        data: &mut D,
        _: <server_sys<D> as ServerBackend<D>>::ClientId,
    ) -> std::io::Result<usize> {
        self.dispatch_events(data)
    }
}

// the tests, one module for each
mod many_args;
mod object_args;
mod protocol_error;
mod server_created_objects;
mod sync;

// some helpers
struct DoNothingData;

impl<D, S: ServerBackend<D>> ClientData<D, S> for DoNothingData {
    fn initialized(&self, _: S::ClientId) {}
    fn disconnected(&self, _: S::ClientId, _: DisconnectReason) {}
}

impl<D, S: ServerBackend<D>> GlobalHandler<D, S> for DoNothingData {
    fn make_data(self: Arc<Self>, _: &mut D, _: &ObjectInfo) -> Arc<dyn ServerObjectData<D, S>> {
        self
    }

    fn bind(&self, _: &mut S::Handle, _: &mut D, _: S::ClientId, _: S::GlobalId, _: S::ObjectId) {}
}

impl<D, S: ServerBackend<D>> ServerObjectData<D, S> for DoNothingData {
    fn make_child(self: Arc<Self>, _: &mut D, _: &ObjectInfo) -> Arc<dyn ServerObjectData<D, S>> {
        self
    }

    fn request(&self, _: &mut S::Handle, _: &mut D, _: S::ClientId, _: Message<S::ObjectId>) {}

    fn destroyed(&self, _: S::ClientId, _: S::ObjectId) {}
}

impl<C: ClientBackend> ClientObjectData<C> for DoNothingData {
    fn make_child(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn ClientObjectData<C>> {
        self
    }

    fn event(&self, _: &mut C::Handle, _: Message<C::ObjectId>) {}

    fn destroyed(&self, _: C::ObjectId) {}
}

struct TestPair<D, C: ClientBackend, S: ServerBackend<D>> {
    pub client: C,
    pub server: S,
    pub client_id: <S as ServerBackend<D>>::ClientId,
}
