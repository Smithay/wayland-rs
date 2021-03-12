#![allow(dead_code, non_snake_case)]

use std::sync::Arc;

use wayland_commons::{
    client::{BackendHandle as ClientHandle, ClientBackend, ObjectData as ClientObjectData},
    server::{
        BackendHandle as ServerHandle, ClientData, CommonPollBackend, DisconnectReason,
        GlobalHandler, IndependentBackend, ObjectData as ServerObjectData,
        ObjectId as ServerObjectId, ServerBackend,
    },
    Argument, ObjectInfo,
};

use wayland_backend_rs::{
    client::Backend as client_rs, server::CommonPollServerBackend as server_common_rs,
    server::IndependentServerBackend as server_independent_rs,
};

use wayland_backend_sys::client::Backend as client_sys;

macro_rules! expand_test {
    ($test_name:ident) => {
        expand_test!(__list_expand, __no_panic, $test_name);
    };
    (panic $test_name:ident) => {
        expand_test!(__list_expand, __panic, $test_name);
    };
    (__list_expand, $panic:tt, $test_name:ident) => {
        expand_test!(__expand, $panic, $test_name, client_rs, server_independent_rs);
        expand_test!(__expand, $panic, $test_name, client_rs, server_common_rs);
        expand_test!(__expand, $panic, $test_name, client_sys, server_independent_rs);
    };
    (__expand, __panic, $test_name: ident, $client_backend: ty, $server_backend: ty) => {
        concat_idents::concat_idents!(fn_name = $test_name, __, $client_backend, __, $server_backend {
            #[test]
            #[should_panic]
            fn fn_name() {
                $test_name::<$client_backend,$server_backend>();
            }
        });
    };
    (__expand, __no_panic, $test_name: ident, $client_backend: ty, $server_backend: ty) => {
        concat_idents::concat_idents!(fn_name = $test_name, __, $client_backend, __, $server_backend {
            #[test]
            fn fn_name() {
                $test_name::<$client_backend,$server_backend>();
            }
        });
    };
}

mod interfaces {
    wayland_scanner::generate_interfaces!("../tests/scanner_assets/test-protocol.xml");
}

impl<C: ClientBackend, S: ServerBackend + ServerPolling<S>> TestPair<C, S> {
    fn init() -> TestPair<C, S> {
        TestPair::init_with_data(Arc::new(DoNothingData))
    }

    fn init_with_data(data: Arc<dyn ClientData<S>>) -> TestPair<C, S> {
        let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();

        let mut server = S::new().unwrap();
        let client_id = server.insert_client(rx, data).unwrap();
        let client = C::connect(tx).unwrap();

        TestPair { client, server, client_id }
    }

    fn client_dispatch(&mut self) -> std::io::Result<usize> {
        self.client.dispatch_events()
    }

    fn client_flush(&mut self) -> std::io::Result<()> {
        self.client.flush()
    }

    fn server_dispatch(&mut self) -> std::io::Result<usize> {
        self.server.poll_client(self.client_id.clone())
    }

    fn server_flush(&mut self) -> std::io::Result<()> {
        self.server.flush(Some(self.client_id.clone()))
    }
}

// A generic trait for polling any kind of server in tests
trait ServerPolling<S: ServerBackend> {
    fn poll_client(&mut self, id: S::ClientId) -> std::io::Result<usize>;
}

// We need to do manual implementations, because otherwise we get conflicting implementations

impl ServerPolling<server_independent_rs> for server_independent_rs {
    fn poll_client(
        &mut self,
        id: <server_independent_rs as ServerBackend>::ClientId,
    ) -> std::io::Result<usize> {
        self.dispatch_events_for(id)
    }
}

impl ServerPolling<server_common_rs> for server_common_rs {
    fn poll_client(
        &mut self,
        _: <server_independent_rs as ServerBackend>::ClientId,
    ) -> std::io::Result<usize> {
        self.dispatch_events()
    }
}

// the tests, one module for each
mod many_args;
mod object_args;
mod sync;

// some helpers
struct DoNothingData;

impl<S: ServerBackend> ClientData<S> for DoNothingData {
    fn initialized(&self, _: S::ClientId) {}
    fn disconnected(&self, _: S::ClientId, _: DisconnectReason) {}
}

impl<S: ServerBackend> GlobalHandler<S> for DoNothingData {
    fn make_data(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn ServerObjectData<S>> {
        self
    }

    fn bind(&self, _: &mut S::Handle, _: S::ClientId, _: S::GlobalId, _: S::ObjectId) {}
}

impl<S: ServerBackend> ServerObjectData<S> for DoNothingData {
    fn make_child(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn ServerObjectData<S>> {
        self
    }

    fn request(
        &self,
        _: &mut S::Handle,
        _: S::ClientId,
        _: S::ObjectId,
        _: u16,
        _: &[Argument<S::ObjectId>],
    ) {
    }

    fn destroyed(&self, _: S::ClientId, _: S::ObjectId) {}
}

impl<C: ClientBackend> ClientObjectData<C> for DoNothingData {
    fn make_child(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn ClientObjectData<C>> {
        self
    }

    fn event(&self, _: &mut C::Handle, _: C::ObjectId, _: u16, _: &[Argument<C::ObjectId>]) {}

    fn destroyed(&self, _: C::ObjectId) {}
}

struct TestPair<C: ClientBackend, S: ServerBackend> {
    pub client: C,
    pub server: S,
    pub client_id: <S as ServerBackend>::ClientId,
}
