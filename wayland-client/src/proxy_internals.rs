use std::{any::Any, sync::Arc};

use once_cell::sync::OnceCell;

use wayland_backend::{
    client::{Handle, ObjectData, ObjectId},
    protocol::{Message, ObjectInfo},
};

use crate::ConnectionHandle;

#[allow(type_alias_bounds)]
pub(crate) type EventSink = dyn for<'a> Fn(ConnectionHandle<'a>, Message<ObjectId>) + Send + Sync;

pub struct ProxyData {
    sink: Arc<EventSink>,
    udata: OnceCell<Box<dyn Any + Send + Sync>>,
}

impl ProxyData {
    pub fn new(sink: Arc<EventSink>) -> ProxyData {
        ProxyData { sink, udata: OnceCell::new() }
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
        Arc::new(ProxyData::new(self.sink.clone()))
    }
    fn event(&self, handle: &mut Handle, msg: Message<ObjectId>) {
        let cx = ConnectionHandle::from_handle(handle);
        (self.sink)(cx, msg)
    }
    fn destroyed(&self, _object_id: ObjectId) {}
}

#[macro_export]
macro_rules! convert_event {
    ($msg:expr ; $target_iface:ty) => {{
        let msg = $msg;
        let sender_iface = msg.sender_id.interface();
        if $crate::backend::protocol::same_interface(sender_iface, <$target_iface as $crate::Proxy>::interface()) {
            <$target_iface as $crate::Proxy>::parse_event(msg)
                    .map_err(|msg| $crate::DispatchError::BadMessage { msg, interface: sender_iface })
        } else {
            Err($crate::DispatchError::NoHandler { msg, interface: sender_iface })
        }
    }};
    ($msg:expr ; $($target_iface:ty => $convert_closure:expr),* ,?) => {{
        let msg = $msg;
        let sender_iface = msg.sender.interface();
        match () {
            $(
                () if $crate::backend::protocol::same_interface(sender_iface, <$target_iface as $crate::Proxy>::interface()) => {
                    <$target_iface as $crate::Proxy>::parse_event(msg)
                        .map($convert_closure)
                        .map_err(|msg| $crate::DispatchError::BadMessage { msg, interface: sender_iface })
                },
            ),*
            () => Err($crate::DispatchError::NoHandler { msg, interface: sender_iface })
        }
    }};
}
