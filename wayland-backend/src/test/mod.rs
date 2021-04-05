#![allow(dead_code, non_snake_case)]

use std::sync::Arc;

use crate::protocol::{Argument, Message, ObjectInfo};

use crate::rs::{client as client_rs, server as server_rs};
use crate::sys::{client as client_sys, server as server_sys};

macro_rules! expand_test {
    ($test_name:ident, $test_body:tt) => {
        expand_test!(__list_expand, __no_panic, $test_name, $test_body);
    };
    (panic $test_name:ident, $test_body:tt) => {
        expand_test!(__list_expand, __panic, $test_name, $test_body);
    };
    (__list_expand, $panic:tt, $test_name:ident, $test_body:tt) => {
        expand_test!(__expand, $panic, $test_name, client_rs, server_rs, $test_body);
        expand_test!(__expand, $panic, $test_name, client_sys, server_rs, $test_body);
        expand_test!(__expand, $panic, $test_name, client_rs, server_sys, $test_body);
        expand_test!(__expand, $panic, $test_name, client_sys, server_sys, $test_body);
    };
    (__expand, __panic, $test_name: ident, $client_backend: ty, $server_backend: ident, $test_body: tt) => {
        concat_idents::concat_idents!(fn_name = $test_name, __, $client_backend, __, $server_backend {
            #[test]
            #[should_panic]
            fn fn_name() {
                let _ = env_logger::builder().is_test(true).try_init();
                use $client_backend as client_backend;
                use $server_backend as server_backend;
                $test_body
            }
        });
    };
    (__expand, __no_panic, $test_name: ident, $client_backend: ty, $server_backend: ident, $test_body: tt) => {
        concat_idents::concat_idents!(fn_name = $test_name, __, $client_backend, __, $server_backend {
            #[test]
            fn fn_name() {
                let _ = env_logger::builder().is_test(true).try_init();
                use $client_backend as client_backend;
                use $server_backend as server_backend;
                $test_body
            }
        });
    };
}

mod interfaces {
    use crate as wayland_backend;
    wayland_scanner::generate_interfaces!("../tests/scanner_assets/test-protocol.xml");
}

mod many_args;
mod object_args;
mod protocol_error;
mod server_created_objects;
mod sync;

/*
 * A "do nothing" data as a helper
 */
struct DoNothingData;

// Server Client Data

impl<D> server_rs::ClientData<D> for DoNothingData {
    fn initialized(&self, _: server_rs::ClientId) {}
    fn disconnected(&self, _: server_rs::ClientId, _: server_rs::DisconnectReason) {}
}

impl<D> server_sys::ClientData<D> for DoNothingData {
    fn initialized(&self, _: server_sys::ClientId) {}
    fn disconnected(&self, _: server_sys::ClientId, _: server_rs::DisconnectReason) {}
}

// Server Global Handler

impl<D> server_rs::GlobalHandler<D> for DoNothingData {
    fn make_data(self: Arc<Self>, _: &mut D, _: &ObjectInfo) -> Arc<dyn server_rs::ObjectData<D>> {
        self
    }

    fn bind(
        &self,
        _: &mut server_rs::Handle<D>,
        _: &mut D,
        _: server_rs::ClientId,
        _: server_rs::GlobalId,
        _: server_rs::ObjectId,
    ) {
    }
}

impl<D> server_sys::GlobalHandler<D> for DoNothingData {
    fn make_data(self: Arc<Self>, _: &mut D, _: &ObjectInfo) -> Arc<dyn server_sys::ObjectData<D>> {
        self
    }

    fn bind(
        &self,
        _: &mut server_sys::Handle<D>,
        _: &mut D,
        _: server_sys::ClientId,
        _: server_sys::GlobalId,
        _: server_sys::ObjectId,
    ) {
    }
}

// Server Object Data

impl<D> server_rs::ObjectData<D> for DoNothingData {
    fn make_child(self: Arc<Self>, _: &mut D, _: &ObjectInfo) -> Arc<dyn server_rs::ObjectData<D>> {
        self
    }

    fn request(
        &self,
        _: &mut server_rs::Handle<D>,
        _: &mut D,
        _: server_rs::ClientId,
        _: Message<server_rs::ObjectId>,
    ) {
    }

    fn destroyed(&self, _: server_rs::ClientId, _: server_rs::ObjectId) {}
}

impl<D> server_sys::ObjectData<D> for DoNothingData {
    fn make_child(
        self: Arc<Self>,
        _: &mut D,
        _: &ObjectInfo,
    ) -> Arc<dyn server_sys::ObjectData<D>> {
        self
    }

    fn request(
        &self,
        _: &mut server_sys::Handle<D>,
        _: &mut D,
        _: server_sys::ClientId,
        _: Message<server_sys::ObjectId>,
    ) {
    }

    fn destroyed(&self, _: server_sys::ClientId, _: server_sys::ObjectId) {}
}

// Client Object Data

impl client_rs::ObjectData for DoNothingData {
    fn make_child(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn client_rs::ObjectData> {
        self
    }

    fn event(&self, _: &mut client_rs::Handle, _: Message<client_rs::ObjectId>) {}

    fn destroyed(&self, _: client_rs::ObjectId) {}
}

impl client_sys::ObjectData for DoNothingData {
    fn make_child(self: Arc<Self>, _: &ObjectInfo) -> Arc<dyn client_sys::ObjectData> {
        self
    }

    fn event(&self, _: &mut client_sys::Handle, _: Message<client_sys::ObjectId>) {}

    fn destroyed(&self, _: client_sys::ObjectId) {}
}
