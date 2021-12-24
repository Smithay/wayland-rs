use std::sync::{Arc, Mutex};

use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use wayland_backend::{
    client::{Backend, Handle, ObjectData, ObjectId},
    protocol::{AllowNull, ArgumentType, Message},
};

use crate::{ConnectionHandle, DispatchError, Proxy};

/// A trait which provides an implementation for handling events from the server on a proxy with some type of
/// associated user data.
pub trait Dispatch<I: Proxy>: Sized {
    /// The user data associated with the type of proxy.
    type UserData: Send + Sync + 'static;

    /// Called when an event from the server is processed.
    ///
    /// The implementation of this function may vary depending on protocol requirements. Typically the client
    /// will respond to the server by sending requests to the proxy.
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

#[derive(Debug)]
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

#[cfg(not(tarpaulin_include))]
impl<I: Proxy, U: std::fmt::Debug> std::fmt::Debug for QueueProxyData<I, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueProxyData").field("udata", &self.udata).finish()
    }
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

/// The base trait used to define a delegate type to handle some type of proxy.
pub trait DelegateDispatchBase<I: Proxy> {
    /// The type of user data the delegate holds
    type UserData: Send + Sync + 'static;
}

/// A trait which defines a delegate type to handle some type of proxy.
///
/// This trait is useful for building modular handlers of proxies.
///
/// ## Usage
///
/// To explain the trait, let's implement a delegate for handling the events from [`WlRegistry`](crate::protocol::wl_registry::WlRegistry).
///
/// ```
/// # // Maintainers: If this example changes, please make sure you also carry those changes over to the delegate_dispatch macro.
/// use wayland_client::{protocol::wl_registry, DelegateDispatch, DelegateDispatchBase, Dispatch};
///
/// /// The type we want to delegate to
/// struct DelegateToMe;
///
/// // Now implement DelegateDispatchBase.
/// impl DelegateDispatchBase<wl_registry::WlRegistry> for DelegateToMe {
///     /// The type of user data associated with the delegation of events from a registry is defined here.
///     ///
///     /// If you don't need user data, the unit type, `()`, may be used.
///     type UserData = ();
/// }
///
/// // Now implement DelegateDispatch.
/// impl<D> DelegateDispatch<wl_registry::WlRegistry, D> for DelegateToMe
/// where
///     // `D` is the type which has delegated to this type.
///     D: Dispatch<wl_registry::WlRegistry, UserData = Self::UserData>,
/// {
///     fn event(
///         &mut self,
///         _proxy: &wl_registry::WlRegistry,
///         _event: wl_registry::Event,
///         _data: &Self::UserData,
///         _cxhandle: &mut wayland_client::ConnectionHandle,
///         _qhandle: &wayland_client::QueueHandle<D>,
///         _init: &mut wayland_client::DataInit<'_>,
///     ) {
///         // Here the delegate may handle incoming events as it pleases.
///     }
/// }
/// ```
pub trait DelegateDispatch<
    I: Proxy,
    D: Dispatch<I, UserData = <Self as DelegateDispatchBase<I>>::UserData>,
>: Sized + DelegateDispatchBase<I>
{
    /// Called when an event from the server is processed.
    ///
    /// The implementation of this function may vary depending on protocol requirements. Typically the client
    /// will respond to the server by sending requests to the proxy.
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

/// A helper macro which delegates a set of [`Dispatch`] implementations for a proxy to some other type which
/// implements [`DelegateDispatch`] for each proxy.
///
/// This macro allows more easily delegating smaller parts of the protocol an application may wish to handle
/// in a modular fashion.
///
/// # Usage
///
/// For example, say you want to delegate events for [`WlRegistry`](crate::protocol::wl_registry::WlRegistry)
/// to some other type.
///
/// For brevity, we will use the example in the documentation for [`DelegateDispatch`], `DelegateToMe`.
///
/// ```
/// use wayland_client::{delegate_dispatch, protocol::wl_registry};
/// #
/// # use wayland_client::{DelegateDispatch, DelegateDispatchBase, Dispatch};
/// #
/// # struct DelegateToMe;
/// #
/// # impl DelegateDispatchBase<wl_registry::WlRegistry> for DelegateToMe {
/// #     type UserData = ();
/// # }
/// #
/// # impl<D> DelegateDispatch<wl_registry::WlRegistry, D> for DelegateToMe
/// # where
/// #     D: Dispatch<wl_registry::WlRegistry, UserData = Self::UserData>,
/// # {
/// #     fn event(
/// #         &mut self,
/// #         _proxy: &wl_registry::WlRegistry,
/// #         _event: wl_registry::Event,
/// #         _data: &Self::UserData,
/// #         _cxhandle: &mut wayland_client::ConnectionHandle,
/// #         _qhandle: &wayland_client::QueueHandle<D>,
/// #         _init: &mut wayland_client::DataInit<'_>,
/// #     ) {
/// #     }
/// # }
///
/// // ExampleApp is the type events will be dispatched to.
///
/// /// The application state
/// struct ExampleApp {
///     /// The delegate for handling wl_registry events.
///     delegate: DelegateToMe,
/// }
///
/// // Use delegate_dispatch to implement Dispatch<wl_registry::WlRegistry> for ExampleApp.
/// delegate_dispatch!(ExampleApp: [wl_registry::WlRegistry] => DelegateToMe ; |app| {
///     // Return an `&mut` reference to the field the Dispatch implementation provided by the macro should
///     // forward events to.
///     // You may also use a function to get the delegate since this is like a closure.
///     &mut app.delegate
/// });
///
/// // To explain the macro above, you may read it as the following:
/// //
/// // For ExampleApp, delegate WlRegistry to DelegateToMe and use the closure to get an `&mut` reference to
/// // the delegate.
///
/// // Assert ExampleApp can Dispatch events for wl_registry
/// fn assert_is_registry_delegate<T>()
/// where
///     T: Dispatch<wl_registry::WlRegistry, UserData = ()>,
/// {
/// }
///
/// assert_is_registry_delegate::<ExampleApp>();
/// ```
///
/// You may also delegate multiple proxies to a single type. This is especially useful for handling multiple
/// related protocols in the same modular component.
///
/// For example, a type which can dispatch both the `wl_output` and `xdg_output` protocols may be used as a
/// delegate:
///
/// ```ignore
/// # // This is not tested because xdg_output is in wayland-protocols.
/// delegate_dispatch!(ExampleApp: [wl_output::WlOutput, xdg_output::XdgOutput] => OutputDelegate ; |app| {
///     &mut app.output_delegate
/// });
/// ```
///
/// If your delegate contains a lifetime, you will need to explicitly declare the user data type and
/// use the anonymous lifetime.
///
/// ```
/// use std::marker::PhantomData;
///
/// use wayland_client::{delegate_dispatch, DelegateDispatch, DelegateDispatchBase, Dispatch, protocol::wl_registry};
///
/// struct ExampleApp;
///
/// struct DelegateWithLifetime<'a>(PhantomData<&'a mut ()>);
///
/// // ... DelegateDispatch impl here...
/// # impl DelegateDispatchBase<wl_registry::WlRegistry> for DelegateWithLifetime<'_> {
/// #     type UserData = ();
/// # }
/// # impl<D> DelegateDispatch<wl_registry::WlRegistry, D> for DelegateWithLifetime<'_>
/// # where
/// #     D: Dispatch<wl_registry::WlRegistry, UserData = Self::UserData>,
/// # {
/// #     fn event(
/// #         &mut self,
/// #         _proxy: &wl_registry::WlRegistry,
/// #         _event: wl_registry::Event,
/// #         _data: &Self::UserData,
/// #         _cxhandle: &mut wayland_client::ConnectionHandle,
/// #         _qhandle: &wayland_client::QueueHandle<D>,
/// #         _init: &mut wayland_client::DataInit<'_>,
/// #     ) {
/// #     }
/// # }
///
/// delegate_dispatch!(ExampleApp: <UserData = ()> [wl_registry::WlRegistry] => DelegateWithLifetime<'_> ; |_app| {
///     &mut DelegateWithLifetime(PhantomData)
/// });
/// ```
#[macro_export]
macro_rules! delegate_dispatch {
    ($dispatch_from: ty: [$($interface: ty),*] => $dispatch_to: ty ; |$dispatcher: ident| $closure: block) => {
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
                    let $dispatcher = self; // We need to do this so the closure can see the dispatcher field.
                    let delegate: &mut $dispatch_to = { $closure };
                    $crate::DelegateDispatch::<$interface, _>::event(delegate, proxy, event, data, cxhandle, qhandle, init)
                }
            }
        )*
    };

    // Explicitly specify the UserData if there is a lifetime.
    ($dispatch_from: ty: <UserData = $user_data: ty> [$($interface: ty),*] => $dispatch_to: ty ; |$dispatcher: ident| $closure: block) => {
        $(
            impl $crate::Dispatch<$interface> for $dispatch_from {
                type UserData = $user_data;

                fn event(
                    &mut self,
                    proxy: &$interface,
                    event: <$interface as $crate::Proxy>::Event,
                    data: &Self::UserData,
                    cxhandle: &mut $crate::ConnectionHandle,
                    qhandle: &$crate::QueueHandle<Self>,
                    init: &mut $crate::DataInit<'_>,
                ) {
                    let $dispatcher = self; // We need to do this so the closure can see the dispatcher field.
                    let delegate: &mut $dispatch_to = { $closure };
                    $crate::DelegateDispatch::<$interface, _>::event(delegate, proxy, event, data, cxhandle, qhandle, init)
                }
            }
        )*
    };
}
