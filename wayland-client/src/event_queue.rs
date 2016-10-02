use std::any::Any;
use std::io::{Result as IoResult, Error as IoError};
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_void, c_int};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use wayland_sys::client::*;
use wayland_sys::common::*;
use {Handler, Proxy};

type ProxyUserData = (*mut EventQueueHandle, Arc<AtomicBool>);

/// Handle to an event queue
///
/// This handle gives you access to methods on an event queue
/// that are safe to do from within a callback.
///
/// They are also available on an `EventQueue` object via `Deref`.
pub struct EventQueueHandle {
    handlers: Vec<Box<Any>>
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
    pub fn register<P: Proxy, H: Handler<P> + Any + 'static>(&mut self, proxy: &P, handler_id: usize) {
        let h = self.handlers[handler_id].downcast_ref::<H>()
                    .expect("Handler type do not match.");
        unsafe {
            let data: *mut ProxyUserData = ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_proxy_get_user_data,
                proxy.ptr()
            ) as *mut _;
            (&mut *data).0 = self as *const _ as *mut _;
            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_proxy_add_dispatcher,
                proxy.ptr(),
                dispatch_func::<P,H>,
                h as *const _ as *const c_void,
                data as *mut c_void
            );
        }
    }

    /// Insert a new handler to this event queue
    ///
    /// Returns the index of this handler in the internal array, which is needed
    /// to register proxies to it.
    pub fn add_handler<H: Any + 'static>(&mut self, handler: H) -> usize {
        self.handlers.push(Box::new(handler) as Box<Any>);
        self.handlers.len() - 1
    }

    /// Insert a new handler with init
    ///
    /// Allows you to insert handlers that require some interaction with the
    /// event loop in their initialization, like registering some objects to it.
    ///
    /// The handler must implement the `Init` trait, and its init method will
    /// be called after its insertion.
    pub fn add_handler_with_init<H: Init + Any + 'static>(&mut self, handler: H) -> usize
    {
        let mut box_ = Box::new(handler);
        // this little juggling is to avoid the double-borrow, which is actually safe,
        // as handlers cannot be mutably accessed outside of an event-dispatch,
        // and this new handler cannot receive any events before the return
        // of this function
        let h = &mut *box_ as *mut H;
        self.handlers.push(box_ as Box<Any>);
        let index = self.handlers.len() - 1;
        unsafe { (&mut *h).init(self, index) };
        index
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
    evq: &'evq mut EventQueue
}

impl<'evq> StateGuard<'evq> {
    /// Get a reference to a handler
    ///
    /// Provides a reference to a handler stored in this event loop.
    ///
    /// The H type must be provided and match the type of the targetted Handler, or
    /// it will panic.
    pub fn get_handler<H: Any + 'static>(&self, handler_id: usize) -> &H {
        self.evq.handle.handlers[handler_id].downcast_ref::<H>()
            .expect("Handler type do not match.")
    }

    /// Get a mutable reference to a handler
    ///
    /// Provides a reference to a handler stored in this event loop.
    ///
    /// The H type must be provided and match the type of the targetted Handler, or
    /// it will panic.
    pub fn get_mut_handler<H: Any + 'static>(&mut self, handler_id: usize) -> &mut H {
        self.evq.handle.handlers[handler_id].downcast_mut::<H>()
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
/// via the `&mut self` argument. You can then instanciate your type, and give ownership of
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
    display: *mut wl_display,
    wlevq: Option<*mut wl_event_queue>,
    handle: Box<EventQueueHandle>
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
    /// If an error is returned, your connexion with the wayland
    /// compositor is probably lost.
    pub fn dispatch(&mut self) -> IoResult<u32> {
        let ret = match self.wlevq {
            Some(evq) => unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_dispatch_queue,
                    self.display,
                    evq
                )
            },
            None => unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_dispatch,
                    self.display
                )
            }
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
    /// If an error is returned, your connexion with the wayland
    /// compositor is probably lost.
    pub fn dispatch_pending(&mut self) -> IoResult<u32> {
        let ret = match self.wlevq {
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
            }
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
        let ret = unsafe { match self.wlevq {
            Some(evtq) => {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_roundtrip_queue,
                    self.display, evtq)
            }
            None => {
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_roundtrip, self.display)
            }
        }};
        if ret >= 0 { Ok(ret) } else { Err(IoError::last_os_error()) }
    }

    /// Get a handle to the internal state
    ///
    /// The returned guard object allows you to get references
    /// to the handler objects you previously inserted in this
    /// event queue.
    pub fn state(&mut self) -> StateGuard {
        StateGuard { evq: self }
    }
}

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

pub unsafe fn create_event_queue(display: *mut wl_display, evq: Option<*mut wl_event_queue>) -> EventQueue {
    EventQueue {
        display: display,
        wlevq: evq,
        handle: Box::new(EventQueueHandle {
            handlers: Vec::new()
        })
    }
}

unsafe extern "C" fn dispatch_func<P: Proxy, H: Handler<P>>(
    handler: *const c_void,
    proxy: *mut c_void,
    opcode: u32,
    _msg: *const wl_message,
    args: *const wl_argument
) -> c_int {
    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        // This cast from *const to *mut is legit because we enforce that a Handler
        // can only be assigned to a single EventQueue.
        // (this is actually the whole point of the design of this lib)
        let handler = &mut *(handler as *const H as *mut H);
        let proxy = P::from_ptr_initialized(proxy as *mut wl_proxy);
        let data = &mut *(ffi_dispatch!(
            WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, proxy.ptr()
        ) as *mut ProxyUserData);
        let evqhandle = &mut *data.0;
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
#[macro_export]
macro_rules! declare_handler(
    ($handler_struct: ty, $handler_trait: path, $handled_type: ty) => {
        unsafe impl $crate::Handler<$handled_type> for $handler_struct {
            unsafe fn message(&mut self, evq: &mut $crate::EventQueueHandle, proxy: &$handled_type, opcode: u32, args: *const $crate::sys::wl_argument) -> Result<(),()> {
                <$handler_struct as $handler_trait>::__message(self, evq, proxy, opcode, args)
            }
        }
    }
);
