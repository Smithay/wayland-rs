use std::{any::Any, sync::Arc};

use once_cell::sync::OnceCell;

use wayland_backend::{
    client::{Handle, ObjectData, ObjectId},
    protocol::{Message, ObjectInfo},
};

use crate::ConnectionHandle;

#[allow(type_alias_bounds)]
pub(crate) type EventSink = dyn Fn(&'_ mut ConnectionHandle<'_>, Message<ObjectId>) + Send + Sync;

pub struct ProxyData {
    sink: Arc<EventSink>,
    udata: OnceCell<Box<dyn Any + Send + Sync>>,
}

impl std::fmt::Debug for ProxyData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProxyData { .. }")
    }
}

impl ProxyData {
    pub fn new(sink: Arc<EventSink>) -> Arc<ProxyData> {
        Arc::new(ProxyData { sink, udata: OnceCell::new() })
    }

    pub fn init_user_data(&self, f: impl FnOnce() -> Box<dyn Any + Send + Sync>) {
        self.udata.get_or_init(f);
    }

    pub fn get_user_data(&self) -> Option<&(dyn Any + Send + Sync)> {
        self.udata.get().map(|b| &**b)
    }
}

impl ObjectData for ProxyData {
    fn make_child(self: Arc<Self>, _child_info: &ObjectInfo) -> Arc<dyn ObjectData> {
        ProxyData::new(self.sink.clone())
    }
    fn event(&self, handle: &mut Handle, msg: Message<ObjectId>) {
        let mut cx = ConnectionHandle::from_handle(handle);
        (self.sink)(&mut cx, msg)
    }
    fn destroyed(&self, _object_id: ObjectId) {}
}

#[macro_export]
macro_rules! convert_event {
    ($cx: expr, $msg:expr ; $target_iface:ty) => {};
}

#[macro_export]
macro_rules! quick_sink {
    ($target:ty, $callback_body:expr) => {{
        #[inline(always)]
        fn check_type<F: Fn(&mut $crate::ConnectionHandle, <$target as $crate::FromEvent>::Out)>(f: F) -> F { f }
        let callback_body = check_type($callback_body);
        $crate::proxy_internals::ProxyData::new(Arc::new(move |cx, msg| {
            let val = <$target as $crate::FromEvent>::from_event(cx, msg).expect("Unexpected event received in oneshot_sink!");
            callback_body(cx, val)
        }))
    }}
}

#[macro_export]
macro_rules! event_enum {
    (
        $(#[$outer:meta])*
        $sv:vis enum $name:ident {
            $(
                $iface:ty => $variant:ident
            ),*
        }
    ) => {
        $(#[$outer:meta])*
        $sv enum $name {
            $(
                $variant($iface, <$iface as $crate::Proxy>::Event)
            ),*
        }

        impl $crate::FromEvent for $name {
            type Out = $name;
            pub fn parse_event(
                cx: &mut $crate::ConnectionHandle,
                msg: $crate::backend::protocol::Message<$crate::backend::ObjectId>
            ) -> Result<$name, $crate::DispatchError> {
                let sender_iface = msg.sender.interface();
                match () {
                    $(
                        () if $crate::backend::protocol::same_interface(sender_iface, <$iface as $crate::Proxy>::interface()) => {
                            <$iface as $crate::Proxy>::from_event(cx, msg)
                                .map($name::$variant)
                                .map_err(|msg| $crate::DispatchError::BadMessage { msg, interface: sender_iface })
                        },
                    ),*
                    () => Err($crate::DispatchError::NoHandler { msg, interface: sender_iface })
                }
            }
        }
    }
}