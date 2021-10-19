use std::sync::Arc;

use wayland_backend::{
    client::{Handle, ObjectData, ObjectId},
    protocol::{Message, ObjectInfo},
};

use crate::ConnectionHandle;

#[allow(type_alias_bounds)]
pub type EventSink<U> = dyn Fn(&'_ mut ConnectionHandle<'_>, Message<ObjectId>, &U) + Send + Sync;

pub struct ProxyData<U> {
    sink: Arc<EventSink<U>>,
    pub udata: U,
}

impl<U: std::fmt::Debug> std::fmt::Debug for ProxyData<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyData").field("udata", &self.udata).finish_non_exhaustive()
    }
}

impl<U: Default + Send + Sync + 'static> ProxyData<U> {
    pub fn new(sink: Arc<EventSink<U>>) -> Arc<ProxyData<U>> {
        Arc::new(ProxyData { sink, udata: Default::default() })
    }

    pub fn ignore() -> Arc<ProxyData<U>> {
        Self::new(Arc::new(|_, _, _| {}))
    }

    pub fn erase(self: Arc<ProxyData<U>>) -> Arc<dyn ObjectData> {
        self as _
    }
}

impl<U: Default + Send + Sync + 'static> ObjectData for ProxyData<U> {
    fn make_child(self: Arc<Self>, _child_info: &ObjectInfo) -> Arc<dyn ObjectData> {
        ProxyData::<U>::new(self.sink.clone())
    }
    fn event(&self, handle: &mut Handle, msg: Message<ObjectId>) {
        let mut cx = ConnectionHandle::from_handle(handle);
        (self.sink)(&mut cx, msg, &self.udata)
    }
    fn destroyed(&self, _object_id: ObjectId) {}
}

#[macro_export]
macro_rules! quick_sink {
    ($target:ty, $callback_body:expr) => {{
        #[inline(always)]
        fn check_type<
            U,
            F: Fn(&mut $crate::ConnectionHandle, $target, <$target as $crate::Proxy>::Event, &U),
        >(
            f: F,
        ) -> F {
            f
        }
        let callback_body = check_type($callback_body);
        $crate::proxy_internals::ProxyData::new(Arc::new(move |cx, msg, udata| {
            let (proxy, event) = <$target as $crate::Proxy>::parse_event(cx, msg)
                .expect("Unexpected event received in oneshot_sink!");
            callback_body(cx, proxy, event, udata)
        }))
    }};
}
