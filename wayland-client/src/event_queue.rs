use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use slotmap::{DefaultKey, SlotMap};
use wayland_backend::{
    client::{Backend, ObjectId},
    protocol::Message,
};

use crate::{proxy_internals::ProxyData, ConnectionHandle, DispatchError, FromEvent, Proxy};

pub trait Dispatch<I: Proxy>: Sized {
    type UserData: Default + Send + Sync + 'static;

    fn event(
        &mut self,
        proxy: I,
        event: I::Event,
        data: &Self::UserData,
        cxhandle: &mut ConnectionHandle,
        qhandle: &QueueHandle<Self>,
    );

    fn destroyed(&mut self, _proxy: I, _data: &Self::UserData) {}
}

#[derive(Debug)]
enum QueueEvent {
    Msg(DefaultKey, Message<ObjectId>),
    SinkDropped(DefaultKey),
}

type InnerCallback<D> = dyn FnMut(
    &mut ConnectionHandle<'_>,
    Message<ObjectId>,
    &mut D,
    &QueueHandle<D>,
) -> Result<(), DispatchError>;

struct QueueCallback<Data>(Rc<RefCell<InnerCallback<Data>>>);

impl<Data> Clone for QueueCallback<Data> {
    fn clone(&self) -> Self {
        QueueCallback(self.0.clone())
    }
}

impl<D> QueueCallback<D> {
    fn new<
        F: FnMut(
                &mut ConnectionHandle<'_>,
                Message<ObjectId>,
                &mut D,
                &QueueHandle<D>,
            ) -> Result<(), DispatchError>
            + 'static,
    >(
        f: F,
    ) -> Self {
        QueueCallback(Rc::new(RefCell::new(f)) as Rc<_>)
    }

    fn invoke(
        &self,
        handle: &mut ConnectionHandle,
        msg: Message<ObjectId>,
        data: &mut D,
        qhandle: &QueueHandle<D>,
    ) -> Result<(), DispatchError> {
        let mut guard = self.0.borrow_mut();
        (*guard)(handle, msg, data, qhandle)
    }
}

impl<D> std::fmt::Debug for QueueCallback<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EventQueueCallback{ .. }")
    }
}

pub struct EventQueue<Data> {
    rx: UnboundedReceiver<QueueEvent>,
    handle: QueueHandle<Data>,
    backend: Arc<Mutex<Backend>>,
}

impl<Data> std::fmt::Debug for EventQueue<Data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventQueue").field("rx", &self.rx).field("handle", &self.handle).finish()
    }
}

impl<Data> EventQueue<Data> {
    pub(crate) fn new(backend: Arc<Mutex<Backend>>) -> Self {
        let (tx, rx) = unbounded();
        EventQueue {
            rx,
            handle: QueueHandle { tx, callbacks: Rc::new(RefCell::new(SlotMap::new())) },
            backend,
        }
    }

    pub fn handle(&self) -> QueueHandle<Data> {
        self.handle.clone()
    }

    pub fn dispatch_pending(&mut self, data: &mut Data) -> Result<usize, DispatchError> {
        Self::dispatching_impl(&mut self.backend.lock().unwrap(), &mut self.rx, &self.handle, data)
    }

    pub fn blocking_dispatch(&mut self, data: &mut Data) -> Result<usize, DispatchError> {
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
        rx: &mut UnboundedReceiver<QueueEvent>,
        qhandle: &QueueHandle<Data>,
        data: &mut Data,
    ) -> Result<usize, DispatchError> {
        let mut handle = ConnectionHandle::from_handle(backend.handle());
        let mut dispatched = 0;

        while let Ok(Some(evt)) = rx.try_next() {
            match evt {
                QueueEvent::SinkDropped(key) => {
                    qhandle.callbacks.borrow_mut().remove(key);
                }
                QueueEvent::Msg(key, msg) => {
                    let target_cb = qhandle.callbacks.borrow()[key].clone();
                    target_cb.invoke(&mut handle, msg, data, qhandle)?;
                    dispatched += 1;
                }
            }
        }
        Ok(dispatched)
    }
}

pub struct QueueHandle<Data> {
    tx: UnboundedSender<QueueEvent>,
    callbacks: Rc<RefCell<SlotMap<DefaultKey, QueueCallback<Data>>>>,
}

impl<Data> std::fmt::Debug for QueueHandle<Data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueHandle")
            .field("tx", &self.tx)
            .field("callbacks", &self.callbacks)
            .finish()
    }
}

impl<Data> Clone for QueueHandle<Data> {
    fn clone(&self) -> Self {
        QueueHandle { tx: self.tx.clone(), callbacks: self.callbacks.clone() }
    }
}

struct QueueSender {
    key: DefaultKey,
    tx: UnboundedSender<QueueEvent>,
}

impl QueueSender {
    fn send(&self, msg: Message<ObjectId>) {
        if self.tx.unbounded_send(QueueEvent::Msg(self.key, msg)).is_err() {
            log::error!("Event received for EventQueue after it was dropped.");
        }
    }
}

impl Drop for QueueSender {
    fn drop(&mut self) {
        let _ = self.tx.unbounded_send(QueueEvent::SinkDropped(self.key));
    }
}

impl<D> QueueHandle<D> {
    pub fn make_data<I: Proxy>(&self) -> Arc<ProxyData<D::UserData>>
    where
        D: Dispatch<I>,
    {
        let callback = QueueCallback::new(move |handle, msg, data: &mut D, qhandle| {
            let (proxy, event) = I::from_event(handle, msg)?;
            let pdata = proxy.data::<D>().expect("Wrong user_data value for object");
            data.event(proxy, event, &pdata.udata, handle, qhandle);
            Ok(())
        });

        let key = self.callbacks.borrow_mut().insert(callback);
        let sender = QueueSender { key, tx: self.tx.clone() };
        let sink_callback = move |_: &mut ConnectionHandle<'_>, msg, _: &D::UserData| {
            sender.send(msg);
        };

        ProxyData::new(Arc::new(sink_callback))
    }
}
