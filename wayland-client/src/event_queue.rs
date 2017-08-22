use {Handler, Proxy};
use std::any::Any;
use std::io::{Error as IoError, Result as IoResult};
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_int, c_void};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr};
use wayland_sys::RUST_MANAGED;

use wayland_sys::client::*;
use wayland_sys::common::*;

type ProxyUserData = (*mut EventQueueHandle, *mut c_void, Arc<(AtomicBool, AtomicPtr<()>)>);

/// Status of a registration attempt of a proxy.
pub enum RegisterStatus {
    /// The proxy was properly registered to this event queue & handler.
    Registered,
    /// The proxy was not registered because it is not managed by `wayland-client`.
    Unmanaged,
    /// The proxy was not registered because it is already destroyed.
    Dead,
}

/// Handle to an event queue
///
/// This handle gives you access to methods on an event queue
/// that are safe to do from within a callback.
///
/// They are also available on an `EventQueue` object via `Deref`.
pub struct EventQueueHandle {
    handlers: Vec<Option<Box<Any + Send>>>,
    wlevq: Option<*mut wl_event_queue>,
}

/// A trait to initialize handlers after they've been inserted in an event queue
///
/// Works with the `add_handler_with_init` method of `EventQueueHandle`.
pub trait Init {
    /// Init the handler
    ///
    /// `index` is the current index of the handler in the event queue (you can
    /// use it to register objects to it)
    fn init(&mut self, evqh: &mut EventQueueHandle, index: usize);
}

impl EventQueueHandle {
    /// Register a proxy to a handler of this event queue.
    ///
    /// The H type must be provided and match the type of the targetted Handler, or
    /// it will panic.
    ///
    /// This overwrites any precedently set Handler for this proxy.
    ///
    /// Returns appropriately and does nothing if this proxy is dead or already managed by
    /// something else than this library.
    pub fn register<P, H>(&mut self, proxy: &P, handler_id: usize) -> RegisterStatus
    where
        P: Proxy,
        H: Handler<P> + Any + Send + 'static,
    {
        let h = self.handlers[handler_id]
            .as_ref()
            .expect("Handler has already been removed.")
            .downcast_ref::<H>()
            .expect("Handler type do not match.");

        match proxy.status() {
            ::Liveness::Dead => return RegisterStatus::Dead,
            ::Liveness::Unmanaged => return RegisterStatus::Unmanaged,
            ::Liveness::Alive => { /* ok, we can continue */ }
        }

        unsafe {
            let data: *mut ProxyUserData =
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, proxy.ptr()) as *mut _;
            // This cast from *const to *mut is legit because we enforce that a Handler
            // can only be assigned to a single EventQueue.
            // (this is actually the whole point of the design of this lib)
            (&mut *data).0 = self as *const _ as *mut _;
            (&mut *data).1 = h as *const _ as *mut c_void;
            // even if this call fails, we updated the user_data, so the new handler is in place.
            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_proxy_add_dispatcher,
                proxy.ptr(),
                dispatch_func::<P, H>,
                &RUST_MANAGED as *const _ as *const _,
                data as *mut c_void
            );
            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_proxy_set_queue,
                proxy.ptr(),
                match self.wlevq {
                    Some(ptr) => ptr,
                    None => ::std::ptr::null_mut(),
                }
            );
        }
        RegisterStatus::Registered
    }

    fn insert_handler(&mut self, h: Box<Any + Send>) -> usize {
        {
            // artificial scope to make the borrow checker happy
            let empty_slot = self.handlers.iter_mut().enumerate().find(|&(_, ref s)| {
                s.is_none()
            });
            if let Some((id, slot)) = empty_slot {
                *slot = Some(h);
                return id;
            }
        }
        self.handlers.push(Some(h));
        self.handlers.len() - 1
    }

    /// Insert a new handler to this event queue
    ///
    /// Returns the index of this handler in the internal array, which is needed
    /// to register proxies to it.
    pub fn add_handler<H: Any + Send + 'static>(&mut self, handler: H) -> usize {
        self.insert_handler(Box::new(handler) as Box<Any + Send>)
    }

    /// Insert a new handler with init
    ///
    /// Allows you to insert handlers that require some interaction with the
    /// event loop in their initialization, like registering some objects to it.
    ///
    /// The handler must implement the `Init` trait, and its init method will
    /// be called after its insertion.
    pub fn add_handler_with_init<H: Init + Any + Send + 'static>(&mut self, handler: H) -> usize {
        let mut box_ = Box::new(handler);
        // this little juggling is to avoid the double-borrow, which is actually safe,
        // as handlers cannot be mutably accessed outside of an event-dispatch,
        // and this new handler cannot receive any events before the return
        // of this function
        let h = &mut *box_ as *mut H;
        let index = self.insert_handler(box_ as Box<Any + Send>);
        unsafe { (&mut *h).init(self, index) };
        index
    }

    /// Remove a handler previously inserted in this event loop and returns it.
    ///
    /// Panics if the requested type does not match the type of the stored handler
    /// or if the specified index was already removed.
    ///
    /// **Unsafety** This function is unsafe because removing a handler while some wayland
    /// objects are still registered to it can lead to access freed memory. Also, the index
    /// of this handler will be reused at next handler insertion.
    pub unsafe fn remove_handler<H: Any + Send + 'static>(&mut self, idx: usize) -> H {
        let is_type = self.handlers[idx]
            .as_ref()
            .expect("Handler has already been removed.")
            .is::<H>();
        assert!(is_type, "Handler type do not match.");
        *(self.handlers[idx].take().unwrap().downcast().unwrap())
    }
}

/// Guard to access internal state of an event queue
///
/// This guard allows you to get references to the handlers you
/// previously stored inside an event queue.
///
/// It borrows the event queue, so no event dispatching is possible
/// as long as the guard is in scope, for safety reasons.
pub struct StateGuard<'evq> {
    evq: &'evq mut EventQueue,
}

impl<'evq> StateGuard<'evq> {
    /// Get a reference to a handler
    ///
    /// Provides a reference to a handler stored in this event loop.
    ///
    /// The H type must be provided and match the type of the targetted Handler, or
    /// it will panic.
    pub fn get_handler<H: Any + 'static>(&self, handler_id: usize) -> &H {
        self.evq.handle.handlers[handler_id]
            .as_ref()
            .expect("Handler has already been removed")
            .downcast_ref::<H>()
            .expect("Handler type do not match.")
    }

    /// Get a mutable reference to a handler
    ///
    /// Provides a reference to a handler stored in this event loop.
    ///
    /// The H type must be provided and match the type of the targetted Handler, or
    /// it will panic.
    pub fn get_mut_handler<H: Any + 'static>(&mut self, handler_id: usize) -> &mut H {
        self.evq.handle.handlers[handler_id]
            .as_mut()
            .expect("Handler has already been removed")
            .downcast_mut::<H>()
            .expect("Handler type do not match.")
    }
}

/// An event queue managing wayland events
///
/// Each wayland object can receive events from the server. To handle these events
/// you must use a handler object: a struct (or enum) which you have implemented
/// the appropriate `Handler` traits on it (each wayland interface defines a `Handler`
/// trait in its module), and declared it using the `declare_handler!(..)` macro.
///
/// This handler contains the state all your handler methods will be able to access
/// via the `&mut self` argument. You can then instantiate your type, and give ownership of
/// the handler object to the event queue, via the `add_handler(..)` method. Then, each
/// wayland object must be registered to a handler via the `register(..)` method (or its events
/// will all be ignored).
///
/// The event queues also provides you control on the flow of the program, via the `dispatch()` and
/// `dispatch_pending()` methods.
///
/// ## example of use
///
/// ```ignore
/// struct MyHandler { /* ... */ }
///
/// impl wl_surface::Handler for MyHandler {
///     // implementation of the handler methods
/// }
///
/// declare_handler!(MyHandler, wl_surface::Handler, wl_surface::WlSurface);
///
/// fn main() {
///     /* ... setup of your environment ... */
///     let surface = compositor.create_surface().expect("Compositor cannot be destroyed.");
///     let my_id = eventqueue.add_handler(MyHandler::new());
///     eventqueue.register::<_, MyHandler>(&surface, my_id);
///
///     // main event loop
///     loop {
///         // flush requests to the server
///         display.flush().unwrap();
///         // dispatch events from the server, blocking if needed
///         eventqueue.dispatch().unwrap();
///     }
/// }
/// ```
pub struct EventQueue {
    handle: Box<EventQueueHandle>,
    display: *mut wl_display,
}

impl EventQueue {
    /// Dispatches events from the internal buffer.
    ///
    /// Dispatches all events to their appropriate handlers.
    /// If not events were in the internal buffer, will block until
    /// some events are read and dispatch them.
    /// This process can insert events in the internal buffers of
    /// other event queues.
    ///
    /// If an error is returned, your connection with the wayland
    /// compositor is probably lost.
    pub fn dispatch(&mut self) -> IoResult<u32> {
        let ret = match self.handle.wlevq {
            Some(evq) => unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_dispatch_queue,
                    self.display,
                    evq
                )
            },
            None => unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch, self.display) },
        };
        if ret >= 0 {
            Ok(ret as u32)
        } else {
            Err(IoError::last_os_error())
        }
    }

    /// Dispatches pending events from the internal buffer.
    ///
    /// Dispatches all events to their appropriate handlers.
    /// Never blocks, if not events were pending, simply returns
    /// `Ok(0)`.
    ///
    /// If an error is returned, your connection with the wayland
    /// compositor is probably lost.
    pub fn dispatch_pending(&mut self) -> IoResult<u32> {
        let ret = match self.handle.wlevq {
            Some(evq) => unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_dispatch_queue_pending,
                    self.display,
                    evq
                )
            },
            None => unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_dispatch_pending,
                    self.display
                )
            },
        };
        if ret >= 0 {
            Ok(ret as u32)
        } else {
            Err(IoError::last_os_error())
        }
    }

    /// Synchronous roundtrip
    ///
    /// This call will cause a synchonous roundtrip with the wayland server. It will block until all
    /// pending requests of this queue are send to the server and it has processed all of them and
    /// send the appropriate events.
    ///
    /// Handlers are called as a consequence.
    ///
    /// On success returns the number of dispatched events.
    pub fn sync_roundtrip(&mut self) -> IoResult<i32> {
        let ret = unsafe {
            match self.handle.wlevq {
                Some(evtq) => {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_roundtrip_queue,
                        self.display,
                        evtq
                    )
                }
                None => ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_roundtrip, self.display),
            }
        };
        if ret >= 0 {
            Ok(ret)
        } else {
            Err(IoError::last_os_error())
        }
    }

    /// Get a handle to the internal state
    ///
    /// The returned guard object allows you to get references
    /// to the handler objects you previously inserted in this
    /// event queue.
    pub fn state(&mut self) -> StateGuard {
        StateGuard { evq: self }
    }

    /// Prepare an conccurent read
    ///
    /// Will declare your intention to read events from the server socket.
    ///
    /// Will return `None` if there are still some events awaiting dispatch on this EventIterator.
    /// In this case, you need to call `dispatch_pending()` before calling this method again.
    ///
    /// As long as the returned guard is in scope, no events can be dispatched to any event iterator.
    ///
    /// The guard can then be destroyed by two means:
    ///
    ///  - Calling its `cancel()` method (or letting it go out of scope): the read intention will
    ///    be cancelled
    ///  - Calling its `read_events()` method: will block until all existing guards are destroyed
    ///    by one of these methods, then events will be read and all blocked `read_events()` calls
    ///    will return.
    ///
    /// This call will otherwise not block on the server socket if it is empty, and return
    /// an io error `WouldBlock` in such cases.
    pub fn prepare_read(&self) -> Option<ReadEventsGuard> {
        let ret = unsafe {
            match self.handle.wlevq {
                Some(evtq) => {
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_display_prepare_read_queue,
                        self.display,
                        evtq
                    )
                }
                None => ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_prepare_read, self.display),
            }
        };
        if ret >= 0 {
            Some(ReadEventsGuard { display: self.display })
        } else {
            None
        }
    }
}

unsafe impl Send for EventQueue {}

impl Deref for EventQueue {
    type Target = EventQueueHandle;
    fn deref(&self) -> &EventQueueHandle {
        &*self.handle
    }
}

impl DerefMut for EventQueue {
    fn deref_mut(&mut self) -> &mut EventQueueHandle {
        &mut *self.handle
    }
}

/// A guard over a read intention.
///
/// See `EventQueue::prepare_read()` for details about its use.
pub struct ReadEventsGuard {
    display: *mut wl_display,
}

impl ReadEventsGuard {
    /// Read events
    ///
    /// Reads events from the server socket. If other `ReadEventsGuard` exists, will block
    /// until they are all consumed or destroyed.
    pub fn read_events(self) -> IoResult<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_read_events, self.display) };
        // Don't run destructor that would cancel the read intent
        ::std::mem::forget(self);
        if ret >= 0 {
            Ok(ret)
        } else {
            Err(IoError::last_os_error())
        }
    }

    /// Cancel the read
    ///
    /// Will cancel the read intention associated with this guard. Never blocks.
    ///
    /// Has the same effet as letting the guard go out of scope.
    pub fn cancel(self) {
        // just run the destructor
    }
}

impl Drop for ReadEventsGuard {
    fn drop(&mut self) {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_cancel_read, self.display) }
    }
}

pub unsafe fn create_event_queue(display: *mut wl_display, evq: Option<*mut wl_event_queue>) -> EventQueue {
    EventQueue {
        display: display,
        handle: Box::new(EventQueueHandle {
            handlers: Vec::new(),
            wlevq: evq,
        }),
    }
}

unsafe extern "C" fn dispatch_func<P: Proxy, H: Handler<P>>(_impl: *const c_void, proxy: *mut c_void,
                                                            opcode: u32, _msg: *const wl_message,
                                                            args: *const wl_argument)
                                                            -> c_int {
    // sanity check, if it triggers, it is a bug
    if _impl != &RUST_MANAGED as *const _ as *const _ {
        let _ = write!(
            ::std::io::stderr(),
            "[wayland-client error] Dispatcher got called for a message on a non-managed object."
        );
        ::libc::abort();
    }
    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let proxy = P::from_ptr_initialized(proxy as *mut wl_proxy);
        let data = &mut *(ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, proxy.ptr()) as
                              *mut ProxyUserData);
        let evqhandle = &mut *data.0;
        let handler = &mut *(data.1 as *mut H);
        handler.message(evqhandle, &proxy, opcode, args)
    });
    match ret {
        Ok(Ok(())) => return 0,   // all went well
        Ok(Err(())) => {
            // an unknown opcode was dispatched, this is not normal
            let _ = write!(
                ::std::io::stderr(),
                "[wayland-client error] Attempted to dispatch unknown opcode {} for {}, aborting.",
                opcode, P::interface_name()
            );
            ::libc::abort();
        }
        Err(_) => {
            // a panic occured
            let _ = write!(
                ::std::io::stderr(),
                "[wayland-client error] A handler for {} panicked, aborting.",
                P::interface_name()
            );
            ::libc::abort();
        }
    }
}

/// Synonym of the declare_handler! macro
///
/// This more distinctive can be used for projects that need to use
/// both the client-side and server-side macros.
#[macro_export]
macro_rules! client_declare_handler(
    ($handler_struct: ident <$($tyarg:ident : [$($trait: ident $(<$($traitarg:ty),*>)*),*]),*>, $handler_trait: path, $handled_type: ty) => {
        unsafe impl<$($tyarg : $($trait $(<$($traitarg),*>)* +)* 'static),*> $crate::Handler<$handled_type> for $handler_struct<$($tyarg),*> {
            unsafe fn message(&mut self,
                              evq: &mut $crate::EventQueueHandle,
                              proxy: &$handled_type,
                              opcode: u32,
                              args: *const $crate::sys::wl_argument
                             ) -> ::std::result::Result<(),()> {
                <$handler_trait>::__message(self, evq, proxy, opcode, args)
            }
        }
    };
    ($handler_struct: ident, $handler_trait: path, $handled_type: ty) => {
        unsafe impl $crate::Handler<$handled_type> for $handler_struct {
            unsafe fn message(&mut self,
                              evq: &mut $crate::EventQueueHandle,
                              proxy: &$handled_type,
                              opcode: u32,
                              args: *const $crate::sys::wl_argument
                             ) -> ::std::result::Result<(),()> {
                <$handler_trait>::__message(self, evq, proxy, opcode, args)
            }
        }
    };
);


/// Registers a handler type so it can be used in event queue
///
/// After having implemented the appropriate Handler trait for your type,
/// declare it via this macro, like this:
///
/// ```ignore
/// struct MyHandler;
///
/// impl wl_foo::Handler for MyHandler {
///     ...
/// }
///
/// declare_handler!(MyHandler, wl_foo::Handler, wl_foo::WlFoo);
/// ```
///
/// If your type has type arguments, they must be specified using this special
/// syntax to describe constraints on them:
///
/// ```ignore
/// // Note that even if there are no constraints on U, there is a need to put this "empty list"
/// declare_handler!(MyHandler<T: [Trait1, Trait2], U: []>, wl_foo::Handler, wl_foo::WlFoo);
/// ```
#[macro_export]
macro_rules! declare_handler(
    ($handler_struct: ident <$($tyarg:ident : [$($trait: ident $(<$($traitarg:ty),*>)*),*]),*>, $handler_trait: path, $handled_type: ty) => {
        client_declare_handler!($handler_struct<$($tyarg: [$($trait $(<$($traitarg),*>)*),*]),*>, $handler_trait, $handled_type);
    };
    ($handler_struct: ident, $handler_trait: path, $handled_type: ty) => {
        client_declare_handler!($handler_struct, $handler_trait, $handled_type);
    };
);

/// Synonym of the declare_delegating_handler! macro
///
/// This more distinctive can be used for projects that need to use
/// both the client-side and server-side macros.
#[macro_export]
macro_rules! client_declare_delegating_handler(
    ($handler_struct: ident <$($tyarg:ident : [$($trait: ident $(<$($traitarg:ty),*>)*),*]),*>, $($handler_field: ident).+ , $handler_trait: path, $handled_type: ty) => {
        unsafe impl<$($tyarg : $($trait $(<$($traitarg),*>)* +)* 'static),*> $crate::Handler<$handled_type> for $handler_struct<$($tyarg),*> {
            unsafe fn message(&mut self,
                              evq: &mut $crate::EventQueueHandle,
                              proxy: &$handled_type,
                              opcode: u32,
                              args: *const $crate::sys::wl_argument
                             ) -> ::std::result::Result<(),()> {
                <$handler_trait>::__message(&mut self.$($handler_field).+, evq, proxy, opcode, args)
            }
        }
    };
    ($handler_struct: ident, $($handler_field: ident).+ , $handler_trait: path, $handled_type: ty) => {
        unsafe impl $crate::Handler<$handled_type> for $handler_struct {
            unsafe fn message(&mut self,
                              evq: &mut $crate::EventQueueHandle,
                              proxy: &$handled_type,
                              opcode: u32,
                              args: *const $crate::sys::wl_argument
                             ) -> ::std::result::Result<(),()> {
                <$handler_trait>::__message(&mut self.$($handler_field).+, evq, proxy, opcode, args)
            }
        }
    };
);

/// Registers a handler type so it as delegating to one of its fields
///
/// This allows to declare your type as a handler, by delegating the impl
/// to one of its fields (or subfields).
///
/// ```ignore
/// // MySubHandler is a proper handler for wl_foo events
/// struct MySubHandler;
///
/// struct MyHandler {
///     sub: MySubHandler
/// }
///
/// declare_delegating_handler!(MySubHandler, sub, wl_foo::Handler, wl_foo::WlFoo);
/// ```
///
/// The syntax to use if your type has type arguments is the same as for `declare_handler!()`.
#[macro_export]
macro_rules! declare_delegating_handler(
    ($handler_struct: ident <$($tyarg:ident : [$($trait: ident $(<$($traitarg:ty),*>)*),*]),*>, $($handler_field: ident).+ , $handler_trait: path, $handled_type: ty) => {
        client_declare_delegating_handler!($handler_struct<$($tyarg: [$($trait $(<$($traitarg),*>)*),*]),*>, $($handler_field).+, $handler_trait, $handled_type);
    };
    ($handler_struct: ident, $($handler_field: ident).+ , $handler_trait: path, $handled_type: ty) => {
        client_declare_delegating_handler!($handler_struct, $($handler_field).+, $handler_trait, $handled_type);
    };
);
