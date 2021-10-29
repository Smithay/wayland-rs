use std::sync::{Arc, Mutex};

use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use wayland_backend::{
    client::{Backend, Handle, ObjectData, ObjectId},
    protocol::{AllowNull, ArgumentType, Message},
};

use crate::{ConnectionHandle, DispatchError, Proxy};

pub trait Dispatch<I: Proxy>: Sized {
    type UserData: Send + Sync + 'static;

    fn event(
        &mut self,
        proxy: &I,
        event: I::Event,
        data: &Self::UserData,
        cxhandle: &mut ConnectionHandle,
        qhandle: &QueueHandle<Self>,
        init: &mut DataInit<'_>,
    );
}

type QueueCallback<D> = fn(
    &mut ConnectionHandle<'_>,
    Message<ObjectId>,
    &mut D,
    &QueueHandle<D>,
) -> Result<(), DispatchError>;

struct QueueEvent<D>(QueueCallback<D>, Message<ObjectId>);

#[cfg(not(tarpaulin_include))]
impl<D> std::fmt::Debug for QueueEvent<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueEvent").field("msg", &self.1).finish_non_exhaustive()
    }
}

pub struct EventQueue<D> {
    rx: UnboundedReceiver<QueueEvent<D>>,
    handle: QueueHandle<D>,
    backend: Arc<Mutex<Backend>>,
}

#[cfg(not(tarpaulin_include))]
impl<D> std::fmt::Debug for EventQueue<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventQueue")
            .field("rx", &self.rx)
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

impl<D> EventQueue<D> {
    pub(crate) fn new(backend: Arc<Mutex<Backend>>) -> Self {
        let (tx, rx) = unbounded();
        EventQueue { rx, handle: QueueHandle { tx }, backend }
    }

    pub fn handle(&self) -> QueueHandle<D> {
        self.handle.clone()
    }

    pub fn dispatch_pending(&mut self, data: &mut D) -> Result<usize, DispatchError> {
        Self::dispatching_impl(&mut self.backend.lock().unwrap(), &mut self.rx, &self.handle, data)
    }

    pub fn blocking_dispatch(&mut self, data: &mut D) -> Result<usize, DispatchError> {
        let mut backend = self.backend.lock().unwrap();
        let dispatched = Self::dispatching_impl(&mut backend, &mut self.rx, &self.handle, data)?;
        if dispatched > 0 {
            Ok(dispatched)
        } else {
            crate::cx::blocking_dispatch_impl(&mut backend)?;
            Self::dispatching_impl(&mut backend, &mut self.rx, &self.handle, data)
        }
    }

    fn dispatching_impl(
        backend: &mut Backend,
        rx: &mut UnboundedReceiver<QueueEvent<D>>,
        qhandle: &QueueHandle<D>,
        data: &mut D,
    ) -> Result<usize, DispatchError> {
        let mut handle = ConnectionHandle::from_handle(backend.handle());
        let mut dispatched = 0;

        while let Ok(Some(QueueEvent(cb, msg))) = rx.try_next() {
            cb(&mut handle, msg, data, qhandle)?;
            dispatched += 1;
        }
        Ok(dispatched)
    }
}

pub struct QueueHandle<D> {
    tx: UnboundedSender<QueueEvent<D>>,
}

#[cfg(not(tarpaulin_include))]
impl<Data> std::fmt::Debug for QueueHandle<Data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueHandle").field("tx", &self.tx).finish()
    }
}

impl<Data> Clone for QueueHandle<Data> {
    fn clone(&self) -> Self {
        QueueHandle { tx: self.tx.clone() }
    }
}

pub(crate) struct QueueSender<D> {
    func: QueueCallback<D>,
    pub(crate) handle: QueueHandle<D>,
}

pub(crate) trait ErasedQueueSender<I> {
    fn send(&self, msg: Message<ObjectId>);
}

impl<I: Proxy, D> ErasedQueueSender<I> for QueueSender<D>
where
    D: Dispatch<I>,
{
    fn send(&self, msg: Message<ObjectId>) {
        if self.handle.tx.unbounded_send(QueueEvent(self.func, msg)).is_err() {
            log::error!("Event received for EventQueue after it was dropped.");
        }
    }
}

impl<D: 'static> QueueHandle<D> {
    pub fn make_data<I: Proxy + 'static>(
        &self,
        user_data: <D as Dispatch<I>>::UserData,
    ) -> Arc<dyn ObjectData>
    where
        D: Dispatch<I>,
    {
        let sender = Box::new(QueueSender { func: queue_callback::<I, D>, handle: self.clone() });
        Arc::new(QueueProxyData { sender, udata: user_data })
    }
}

#[derive(Debug)]
pub struct New<I> {
    id: I,
}

impl<I> New<I> {
    pub fn wrap(id: I) -> New<I> {
        New { id }
    }
}

pub struct DataInit<'a> {
    store: &'a mut Option<(ObjectId, Arc<dyn ObjectData>)>,
}

impl<'a> DataInit<'a> {
    pub fn init<I: Proxy + 'static, D>(
        &mut self,
        resource: New<I>,
        data: <D as Dispatch<I>>::UserData,
        qhandle: &QueueHandle<D>,
    ) -> I
    where
        D: Dispatch<I> + 'static,
    {
        *self.store = Some((resource.id.id(), qhandle.make_data(data)));
        resource.id
    }
}

fn queue_callback<I: Proxy, D: Dispatch<I> + 'static>(
    handle: &mut ConnectionHandle<'_>,
    msg: Message<ObjectId>,
    data: &mut D,
    qhandle: &QueueHandle<D>,
) -> Result<(), DispatchError> {
    let (proxy, event) = I::parse_event(handle, msg)?;
    let udata =
        proxy.data::<<D as Dispatch<I>>::UserData>().expect("Wrong user_data value for object");
    let mut new_data = None;
    data.event(&proxy, event, udata, handle, qhandle, &mut DataInit { store: &mut new_data });
    if let Some((id, data)) = new_data {
        handle.inner.handle().set_data(id, data).unwrap();
    }
    Ok(())
}

pub struct QueueProxyData<I: Proxy, U> {
    pub(crate) sender: Box<dyn ErasedQueueSender<I> + Send + Sync>,
    pub udata: U,
}

impl<I: Proxy + 'static, U: Send + Sync + 'static> ObjectData for QueueProxyData<I, U> {
    fn event(
        self: Arc<Self>,
        _: &mut Handle,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>> {
        let ret = if msg.args.iter().any(|a| a.get_type() == ArgumentType::NewId(AllowNull::No)) {
            Some(Arc::new(TemporaryData) as Arc<dyn ObjectData>)
        } else {
            None
        };
        self.sender.send(msg);
        ret
    }

    fn destroyed(&self, _: ObjectId) {}
}

struct TemporaryData;

impl ObjectData for TemporaryData {
    fn event(self: Arc<Self>, _: &mut Handle, _: Message<ObjectId>) -> Option<Arc<dyn ObjectData>> {
        unreachable!()
    }

    fn destroyed(&self, _: ObjectId) {}
}

/*
 * Dispatch delegation helpers
 */

pub trait DelegateDispatchBase<I: Proxy> {
    type UserData: Default + Send + Sync + 'static;
}

pub trait DelegateDispatch<
    I: Proxy,
    D: Dispatch<I, UserData = <Self as DelegateDispatchBase<I>>::UserData>,
>: Sized + DelegateDispatchBase<I>
{
    fn event(
        &mut self,
        proxy: &I,
        event: I::Event,
        data: &Self::UserData,
        cxhandle: &mut ConnectionHandle,
        qhandle: &QueueHandle<D>,
        init: &mut DataInit<'_>,
    );
}

#[macro_export]
macro_rules! delegate_dispatch {
    ($dispatch_from:ty => $dispatch_to:ty ; [$($interface:ty),*] => $convert:ident) => {
        $(
            impl $crate::Dispatch<$interface> for $dispatch_from {
                type UserData = <$dispatch_to as $crate::DelegateDispatchBase<$interface>>::UserData;

                fn event(
                    &mut self,
                    proxy: &$interface,
                    event: <$interface as $crate::Proxy>::Event,
                    data: &Self::UserData,
                    cxhandle: &mut $crate::ConnectionHandle,
                    qhandle: &$crate::QueueHandle<Self>,
                    init: &mut $crate::DataInit<'_>,
                ) {
                    <$dispatch_to as $crate::DelegateDispatch<$interface, Self>>::event(&mut self.$convert(), proxy, event, data, cxhandle, qhandle, init)
                }
            }
        )*
    }
}
