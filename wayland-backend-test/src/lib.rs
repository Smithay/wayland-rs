#![allow(dead_code, non_snake_case)]

use std::sync::Arc;

use wayland_commons::{
    client::{BackendHandle as ClientHandle, ClientBackend},
    core_interfaces::*,
    server::{
        BackendHandle as ServerHandle, ClientData, CommonPollBackend, DisconnectReason,
        IndependentBackend, ServerBackend,
    },
    Argument, ObjectInfo,
};

use wayland_backend_rs::{
    client::Backend as client_rs, server::IndependentServerBackend as server_independent_rs,
};

macro_rules! expand_test {
    ($test_name:ident) => {
        expand_test!(__expand, $test_name, client_rs, server_independent_rs);
    };
    (__expand, $test_name: ident, $client_backend: ty, $server_backend: ty) => {
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

struct DoNothingClientData;

impl<S: ServerBackend> ClientData<S> for DoNothingClientData {
    fn initialized(&self, _client_id: S::ClientId) {}
    fn disconnected(&self, _client_id: S::ClientId, _reason: DisconnectReason) {}
}

struct TestPair<C: ClientBackend, S: ServerBackend> {
    pub client: C,
    pub server: S,
    pub client_id: <S as ServerBackend>::ClientId,
}

impl<C: ClientBackend, S: ServerBackend + ServerPolling<S>> TestPair<C, S> {
    fn init() -> TestPair<C, S> {
        TestPair::init_with_data(Arc::new(DoNothingClientData))
    }

    fn init_with_data(data: Arc<dyn ClientData<S>>) -> TestPair<C, S> {
        let (tx, rx) = std::os::unix::net::UnixStream::pair().unwrap();

        let mut server = S::new().unwrap();
        let client_id = server.insert_client(rx, data);
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

mod sync;
