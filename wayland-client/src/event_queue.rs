use {Implementable, Proxy};
use std::any::Any;
use std::io::{Error as IoError, Result as IoResult};
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_int, c_void};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr};
pub use token_store::{Store as State, StoreProxy as StateProxy, Token as StateToken};
use wayland_sys::RUST_MANAGED;
use wayland_sys::client::*;
use wayland_sys::common::*;

type ProxyUserData = (
    *mut EventQueueHandle,
    Option<Box<Any>>,
    Arc<(AtomicBool, AtomicPtr<()>)>,
);

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
    state: State,
    wlevq: Option<*mut wl_event_queue>,
}

impl EventQueueHandle {
    /// Register a proxy to this event queue.
    ///
    /// You are required to provide a valid implementation for this proxy
    /// as well as some associated implementation data. This implementation
    /// is expected to be a struct holding the various relevant
    /// function pointers.
    ///
    /// This implementation data can typically contain indexes to state value
    /// that the implementation will need to work on.
    ///
    /// This overwrites any precedently set implementation for this proxy.
    ///
    /// Returns appropriately and does nothing if this proxy is dead or already managed by
    /// something else than this library.
    pub fn register<P, ID>(&mut self, proxy: &P, implementation: P::Implementation, idata: ID)
                           -> RegisterStatus
    where
        P: Proxy + Implementable<ID>,
        ID: 'static,
    {
        match proxy.status() {
            ::Liveness::Dead => return RegisterStatus::Dead,
            ::Liveness::Unmanaged => return RegisterStatus::Unmanaged,
            ::Liveness::Alive => { /* ok, we can continue */ }
        }

        unsafe {
            let data: *mut ProxyUserData =
                ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, proxy.ptr()) as *mut _;
            // This cast from *const to *mut is legit because we enforce that a proxy
            // can only be assigned to a single EventQueue.
            // (this is actually the whole point of the design of this lib)
            (&mut *data).0 = self as *const _ as *mut _;
            (&mut *data).1 = Some(Box::new((implementation, idata)) as Box<Any>);
            // even if this call fails, we updated the user_data, so the new implementation is in place.
            ffi_dispatch!(
                WAYLAND_CLIENT_HANDLE,
                wl_proxy_add_dispatcher,
                proxy.ptr(),
                dispatch_func::<P, ID>,
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

    /// Get a handle to the internal state
    ///
    /// The returned guard object allows you to get references
    /// to the handler objects you previously inserted in this
    /// event queue.
    pub fn state(&mut self) -> &mut State {
        &mut self.state
    }
}

/// An event queue managing wayland events
///
/// Each wayland object can receive events from the server. To handle these events
/// you must associate to these objects an implementation: a struct defined in their
/// respective module, in which you provide a set of functions that will handle each event.
///
/// Your implementation can also access a shared state managed by the event queue. See
/// the `State` struct and the `state()` method on `EventQueueHandle`. If you need this,
/// the way to do it is:
///
/// - insert your state value in the event queue state store, your are then provided with a
///   token to access it
/// - provide this token (you can clone it) as implementation data to any wayland object
///   that need to access this state in its event callbacks.
///
/// The event queues also provides you control on the flow of the program, via the `dispatch()` and
/// `dispatch_pending()` methods.
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
    /// pending requests of this queue are sent to the server and it has processed all of them and
    /// send the appropriate events.
    ///
    /// Handlers are called as a consequence.
    ///
    /// On success returns the number of dispatched events.
    pub fn sync_roundtrip(&mut self) -> IoResult<i32> {
        let ret = unsafe {
            match self.handle.wlevq {
                Some(evtq) => ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_roundtrip_queue,
                    self.display,
                    evtq
                ),
                None => ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_roundtrip, self.display),
            }
        };
        if ret >= 0 {
            Ok(ret)
        } else {
            Err(IoError::last_os_error())
        }
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
                Some(evtq) => ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_prepare_read_queue,
                    self.display,
                    evtq
                ),
                None => ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_prepare_read, self.display),
            }
        };
        if ret >= 0 {
            Some(ReadEventsGuard {
                display: self.display,
            })
        } else {
            None
        }
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
            state: State::new(),
            wlevq: evq,
        }),
    }
}

unsafe extern "C" fn dispatch_func<P, ID>(_impl: *const c_void, proxy: *mut c_void, opcode: u32,
                                          _msg: *const wl_message, args: *const wl_argument)
                                          -> c_int
where
    P: Proxy + Implementable<ID>,
    ID: 'static,
{
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
        proxy.__dispatch_msg(opcode, args)
    });
    match ret {
        Ok(Ok(())) => return 0, // all went well
        Ok(Err(())) => {
            // an unknown opcode was dispatched, this is not normal
            let _ = write!(
                ::std::io::stderr(),
                "[wayland-client error] Attempted to dispatch unknown opcode {} for {}, aborting.",
                opcode,
                P::interface_name()
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
