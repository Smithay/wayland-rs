use std::sync::{Arc, Mutex};

use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use wayland_backend::{
    client::{Backend, Handle, ObjectData, ObjectId, ReadEventsGuard, WaylandError},
    protocol::Message,
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
        connhandle: &mut ConnectionHandle,
        qhandle: &QueueHandle<Self>,
    );

    /// Method used to initialize the user-data of objects created by events
    ///
    /// If the interface does not have any such event, you can ignore it. If not, the
    /// [`event_created_child!`](event_created_child!) macro is provided for overriding it.
    fn event_created_child(opcode: u16, _qhandle: &QueueHandle<Self>) -> Arc<dyn ObjectData> {
        panic!(
            "Missing event_created_child specialization for event opcode {} of {}",
            opcode,
            I::interface().name
        );
    }
}

/// Macro used to override [`Dispatch::event_created_child()`](Dispatch::event_created_child)
///
/// Use this macro inside the [`Dispatch`] implementation to override this method, to implement the
/// initialization of the user data for event-created objects. The usage syntax is as follow:
///
/// ```ignore
/// impl Dispatch<WlFoo> for MyState {
///     type UserData = FooUserData;
///
///     fn event(
///         &mut self,
///         proxy: &WlFoo,
///         event: FooEvent,
///         data: &FooUserData,
///         connhandle: &mut ConnectionHandle,
///         qhandle: &QueueHandle<MyState>
///     ) {
///         /* ... */
///     }
///
///     event_created_child!(MyState, WlFoo, [
///     // there can be multiple lines if this interface has multiple object-creating event
///         2 => (WlBar, BarUserData::new()),
///     //  ~     ~~~~~  ~~~~~~~~~~~~~~~~~~
///     //  |       |       |
///     //  |       |       +-- an expression whose evaluation produces the user data value
///     //  |       +-- the type of the newly created objecy
///     //  +-- the opcode of the event that creates a new object
///     ]);
/// }
/// ```
#[macro_export]
macro_rules! event_created_child {
    ($selftype:ty, $iface:ty, [$($opcode:expr => ($child_iface:ty, $child_udata:expr)),* $(,)?]) => {
        fn event_created_child(
            opcode: u16,
            qhandle: &$crate::QueueHandle<$selftype>
        ) -> std::sync::Arc<dyn $crate::backend::ObjectData> {
            match opcode {
                $(
                    $opcode => {
                        qhandle.make_data::<$child_iface>({$child_udata})
                    },
                )*
                _ => {
                    panic!("Missing event_created_child specialization for event opcode {} of {}", opcode, <$iface as $crate::Proxy>::interface().name);
                },
            }
        }
    }
}

type QueueCallback<D> = fn(
    &mut ConnectionHandle<'_>,
    Message<ObjectId>,
    &mut D,
    Arc<dyn ObjectData>,
    &QueueHandle<D>,
) -> Result<(), DispatchError>;

struct QueueEvent<D>(QueueCallback<D>, Message<ObjectId>, Arc<dyn ObjectData>);

#[cfg(not(tarpaulin_include))]
impl<D> std::fmt::Debug for QueueEvent<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueEvent").field("msg", &self.1).finish_non_exhaustive()
    }
}

/// An event queue
///
/// This is an abstraction for handling event dispatching, that allows you to ensure
/// access to some common state `&mut D` to your event handlers.
///
/// Event queues are created through [`Connection::new_event_queue()`](crate::Connection::new_event_queue).
/// Upon creation, a wayland object is assigned to an event queue by passing the associated [`QueueHandle`]
/// as argument to the method creating it. All event received by that object will be processed by that event
/// queue, when [`dispatch_pending()`](EventQueue::dispatch_pending) or
/// [`blocking_dispatch()`](EventQueue::blocking_dispatch) is invoked.
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

    /// Get a [`QueueHandle`] for this event queue
    pub fn handle(&self) -> QueueHandle<D> {
        self.handle.clone()
    }

    /// Dispatch pending events
    ///
    /// Events are accumulated in the event queue internal buffer when the Wayland socket is read using
    /// the read APIs on [`Connection`](crate::Connection), or when reading is done from an other thread.
    /// This method will dispatch all such pending events by sequentially invoking their associated handlers:
    /// the [`Dispatch`](crate::Dispatch) implementations on the provided `&mut D`.
    pub fn dispatch_pending(&mut self, data: &mut D) -> Result<usize, DispatchError> {
        Self::dispatching_impl(&mut self.backend.lock().unwrap(), &mut self.rx, &self.handle, data)
    }

    /// Block waiting for events and dispatch them
    ///
    /// This method is similar to [`dispatch_pending`](EventQueue::dispatch_pending), but if there are no
    /// pending events it will also block waiting for the Wayland server to send an event.
    ///
    /// A simple app event loop can consist in invoking this method in a loop.
    pub fn blocking_dispatch(&mut self, data: &mut D) -> Result<usize, DispatchError> {
        let dispatched = Self::dispatching_impl(
            &mut self.backend.lock().unwrap(),
            &mut self.rx,
            &self.handle,
            data,
        )?;
        if dispatched > 0 {
            Ok(dispatched)
        } else {
            crate::conn::blocking_dispatch_impl(self.backend.clone())?;
            Self::dispatching_impl(
                &mut self.backend.lock().unwrap(),
                &mut self.rx,
                &self.handle,
                data,
            )
        }
    }

    /// Start a synchronized read from the socket
    ///
    /// This is needed if you plan to wait on readiness of the Wayland socket using an event
    /// loop. See [`ReadEventsGuard`] for details. Once the events are received, you'll then
    /// need to dispatch them from the event queue using
    /// [`EventQueue::dispatch_pending()`](EventQueue::dispatch_pending).
    ///
    /// If you don't need to manage multiple event sources, see
    /// [`blocking_dispatch()`](EventQueue::blocking_dispatch) for a simpler mechanism.
    pub fn prepare_read(&self) -> Result<ReadEventsGuard, WaylandError> {
        ReadEventsGuard::try_new(self.backend.clone())
    }

    /// Flush pending outgoing events to the server
    ///
    /// This needs to be done regularly to ensure the server receives all your requests.
    pub fn flush(&self) -> Result<(), WaylandError> {
        self.backend.lock().unwrap().flush()
    }

    fn dispatching_impl(
        backend: &mut Backend,
        rx: &mut UnboundedReceiver<QueueEvent<D>>,
        qhandle: &QueueHandle<D>,
        data: &mut D,
    ) -> Result<usize, DispatchError> {
        let mut handle = ConnectionHandle::from_handle(backend.handle());
        let mut dispatched = 0;

        while let Ok(Some(QueueEvent(cb, msg, odata))) = rx.try_next() {
            cb(&mut handle, msg, data, odata, qhandle)?;
            dispatched += 1;
        }
        Ok(dispatched)
    }
}

/// A handle representing an [`EventQueue`], used to assign objects upon creation.
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
    fn send(&self, msg: Message<ObjectId>, odata: Arc<dyn ObjectData>);
}

impl<I: Proxy, D> ErasedQueueSender<I> for QueueSender<D>
where
    D: Dispatch<I>,
{
    fn send(&self, msg: Message<ObjectId>, odata: Arc<dyn ObjectData>) {
        if self.handle.tx.unbounded_send(QueueEvent(self.func, msg, odata)).is_err() {
            log::error!("Event received for EventQueue after it was dropped.");
        }
    }
}

impl<D: 'static> QueueHandle<D> {
    /// Create an object data associated with this event queue
    ///
    /// This creates an implementation of [`ObjectData`] fitting for direct use with `wayland-backend` APIs
    /// that forwards all events to the event queue associated with this token, integrating the object into
    /// the [`Dispatch`]-based logic of `wayland-client`.
    pub fn make_data<I: Proxy + 'static>(
        &self,
        user_data: <D as Dispatch<I>>::UserData,
    ) -> Arc<dyn ObjectData>
    where
        D: Dispatch<I>,
    {
        let sender = Box::new(QueueSender { func: queue_callback::<I, D>, handle: self.clone() });

        let has_creating_event =
            I::interface().events.iter().any(|desc| desc.child_interface.is_some());

        let odata_maker = if has_creating_event {
            let qhandle = self.clone();
            Box::new(move |msg: &Message<ObjectId>| {
                Some(<D as Dispatch<I>>::event_created_child(msg.opcode, &qhandle))
            }) as Box<_>
        } else {
            Box::new(|_: &Message<ObjectId>| None) as Box<_>
        };
        Arc::new(QueueProxyData { sender, odata_maker, udata: user_data })
    }
}

fn queue_callback<I: Proxy + 'static, D: Dispatch<I> + 'static>(
    handle: &mut ConnectionHandle<'_>,
    msg: Message<ObjectId>,
    data: &mut D,
    odata: Arc<dyn ObjectData>,
    qhandle: &QueueHandle<D>,
) -> Result<(), DispatchError> {
    let (proxy, event) = I::parse_event(handle, msg)?;
    let proxy_data = (&*odata)
        .downcast_ref::<QueueProxyData<I, <D as Dispatch<I>>::UserData>>()
        .expect("Wrong user_data value for object");
    data.event(&proxy, event, &proxy_data.udata, handle, qhandle);
    Ok(())
}

type ObjectDataFactory = dyn Fn(&Message<ObjectId>) -> Option<Arc<dyn ObjectData>> + Send + Sync;

/// The [`ObjectData`] implementation used by Wayland proxies, integrating with [`Dispatch`]
pub struct QueueProxyData<I: Proxy, U> {
    pub(crate) sender: Box<dyn ErasedQueueSender<I> + Send + Sync>,
    odata_maker: Box<ObjectDataFactory>,
    /// The user data associated with this object
    pub udata: U,
}

impl<I: Proxy + 'static, U: Send + Sync + 'static> ObjectData for QueueProxyData<I, U> {
    fn event(
        self: Arc<Self>,
        _: &mut Handle,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>> {
        let ret = (self.odata_maker)(&msg);
        self.sender.send(msg, self.clone());
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
///     // If your delegate type has some internal state, it'll need to access it, and you can
///     // require it via an AsMut<_> implementation for example
///     D: AsMut<DelegateToMe>,
/// {
///     fn event(
///         data: &mut D,
///         _proxy: &wl_registry::WlRegistry,
///         _event: wl_registry::Event,
///         _udata: &Self::UserData,
///         _connhandle: &mut wayland_client::ConnectionHandle,
///         _qhandle: &wayland_client::QueueHandle<D>,
///     ) {
///         // Here the delegate may handle incoming events as it pleases.
///
///         // For example, it retrives its state and does some processing with it
///         let me: &mut DelegateToMe = data.as_mut();
///         // do something with `me` ...
/// #       std::mem::drop(me) // use `me` to avoid a warning
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
        data: &mut D,
        proxy: &I,
        event: I::Event,
        udata: &Self::UserData,
        connhandle: &mut ConnectionHandle,
        qhandle: &QueueHandle<D>,
    );

    /// Method used to initialize the user-data of objects created by events
    ///
    /// If the interface does not have any such event, you can ignore it. If not, the
    /// [`event_created_child!`](event_created_child!) macro is provided for overriding it.
    fn event_created_child(opcode: u16, _qhandle: &QueueHandle<D>) -> Arc<dyn ObjectData> {
        panic!(
            "Missing event_created_child specialization for event opcode {} of {}",
            opcode,
            I::interface().name
        );
    }
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
/// #     D: Dispatch<wl_registry::WlRegistry, UserData = Self::UserData> + AsMut<DelegateToMe>,
/// # {
/// #     fn event(
/// #         _data: &mut D,
/// #         _proxy: &wl_registry::WlRegistry,
/// #         _event: wl_registry::Event,
/// #         _udata: &Self::UserData,
/// #         _connhandle: &mut wayland_client::ConnectionHandle,
/// #         _qhandle: &wayland_client::QueueHandle<D>,
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
/// delegate_dispatch!(ExampleApp: [wl_registry::WlRegistry] => DelegateToMe);
///
/// // But DelegateToMe requires that ExampleApp implements AsMut<DelegateToMe>, so we provide this impl
/// impl AsMut<DelegateToMe> for ExampleApp {
///     fn as_mut(&mut self) -> &mut DelegateToMe {
///         &mut self.delegate
///     }
/// }
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
/// delegate_dispatch!(ExampleApp: [wl_output::WlOutput, xdg_output::XdgOutput] => OutputDelegate);
/// ```
#[macro_export]
macro_rules! delegate_dispatch {
    ($dispatch_from: ty: [$($interface: ty),*] => $dispatch_to: ty) => {
        $(
            impl $crate::Dispatch<$interface> for $dispatch_from {
                type UserData = <$dispatch_to as $crate::DelegateDispatchBase<$interface>>::UserData;

                fn event(
                    &mut self,
                    proxy: &$interface,
                    event: <$interface as $crate::Proxy>::Event,
                    data: &Self::UserData,
                    connhandle: &mut $crate::ConnectionHandle,
                    qhandle: &$crate::QueueHandle<Self>,
                ) {
                    <$dispatch_to as $crate::DelegateDispatch<$interface, Self>>::event(self, proxy, event, data, connhandle, qhandle)
                }

                fn event_created_child(
                    opcode: u16,
                    qhandle: &$crate::QueueHandle<Self>
                ) -> ::std::sync::Arc<dyn $crate::backend::ObjectData> {
                    <$dispatch_to as $crate::DelegateDispatch<$interface, Self>>::event_created_child(opcode, qhandle)
                }
            }
        )*
    };
}
