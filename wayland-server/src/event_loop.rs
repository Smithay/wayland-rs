use {Client, Implementable, Resource};
use std::any::Any;
use std::cell::RefCell;
use std::io::{Error as IoError, Result as IoResult};
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_int, c_void};
use std::os::unix::io::RawFd;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr};
pub use token_store::{Store as State, StoreProxy as StateProxy, Token as StateToken};
use wayland_sys::RUST_MANAGED;
use wayland_sys::common::{wl_argument, wl_message};
use wayland_sys::server::*;

type ResourceUserData<R> = (
    *mut EventLoopHandle,
    Option<Box<Any>>,
    Arc<(AtomicBool, AtomicPtr<()>)>,
    Option<fn(&R)>,
);

/// Status of a registration attempt of a resource.
pub enum RegisterStatus {
    /// The resource was properly registered to this event loop & handler.
    Registered,
    /// The resource was not registered because it is not managed by `wayland-server`.
    Unmanaged,
    /// The resource was not registered because it is already destroyed.
    Dead,
}

/// A handle to a global object
///
/// This is given to you when you register a global to the event loop.
///
/// This handle allows you do destroy the global when needed.
///
/// If you know you will never destroy this global, you can let this
/// handle go out of scope.
pub struct Global<R, ID> {
    ptr: *mut wl_global,
    data: *mut (GlobalCallback<R, ID>, *mut EventLoopHandle, ID),
}

impl<R, ID> Global<R, ID> {
    /// Destroy the associated global object.
    pub fn destroy(self) {
        unsafe {
            // destroy the global
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_destroy, self.ptr);
            // free the user data
            let data = Box::from_raw(self.data);
            drop(data);
        }
    }
}

/// Handle to an event loop
///
/// This handle gives you access to methods on an event loop
/// that are safe to do from within a callback.
///
/// They are also available on an `EventLoop` object via `Deref`.
pub struct EventLoopHandle {
    state: State,
    keep_going: bool,
    ptr: *mut wl_event_loop,
    display: Option<*mut wl_display>,
}

impl EventLoopHandle {
    /// Register a resource to this event loop.
    ///
    /// You are required to provide a valid implementation for this proxy
    /// as well as some associated implementation data. This implementation
    /// is expected to be a struct holding the various relevant
    /// function pointers.
    ///
    /// This implementation data can typically contain indexes to state value
    /// that the implementation will need to work on.
    ///
    /// If you provide a destructor function, it will be called whenever the resource
    /// is destroyed, be it at the client request or because the associated client
    /// was disconnected. You'd typically use this to cleanup resources
    ///
    /// This overwrites any precedently set implementation for this proxy.
    ///
    /// Returns appropriately and does nothing if this proxy is dead or already managed by
    /// something else than this library.
    pub fn register<R, ID>(&mut self, resource: &R, implementation: R::Implementation, idata: ID,
                           destructor: Option<fn(&R)>)
                           -> RegisterStatus
    where
        R: Resource + Implementable<ID>,
        ID: 'static,
    {
        match resource.status() {
            ::Liveness::Dead => return RegisterStatus::Dead,
            ::Liveness::Unmanaged => return RegisterStatus::Unmanaged,
            ::Liveness::Alive => { /* ok, we can continue */ }
        }

        unsafe {
            let data: *mut ResourceUserData<R> = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_get_user_data,
                resource.ptr()
            ) as *mut _;
            // This cast from *const to *mut is legit because we enforce that a Handler
            // can only be assigned to a single EventQueue.
            // (this is actually the whole point of the design of this lib)
            (&mut *data).0 = self as *const _ as *mut _;
            (&mut *data).1 = Some(Box::new((implementation, idata)) as Box<Any>);
            (&mut *data).3 = destructor;
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_set_dispatcher,
                resource.ptr(),
                dispatch_func::<R, ID>,
                &RUST_MANAGED as *const _ as *const _,
                data as *mut c_void,
                Some(resource_destroy::<R>)
            );
        }
        RegisterStatus::Registered
    }

    /// Stop looping
    ///
    /// If the event loop this handle belongs to is currently running its `run()`
    /// method, it'll stop and return as soon as the current dispatching session ends.
    pub fn stop_loop(&mut self) {
        self.keep_going = false;
    }

    /// Get an handle to the internal state
    ///
    /// The returned guard object allows you to get references
    /// to the handler objects you previously inserted in this
    /// event loop.
    pub fn state(&mut self) -> &mut State {
        &mut self.state
    }

    /// Add a File Descriptor event source to this event loop
    ///
    /// The interest in read/write capability for this FD must be provided
    /// (and can be changed afterwards using the returned object), and the
    /// associated handler will be called whenever these capabilities are
    /// satisfied, during the dispatching of this event loop.
    pub fn add_fd_event_source<ID: 'static>(&mut self, fd: RawFd,
                                            implementation: ::event_sources::FdEventSourceImpl<ID>,
                                            idata: ID, interest: ::event_sources::FdInterest)
                                            -> IoResult<::event_sources::FdEventSource<ID>> {
        let data = Box::new((
            implementation,
            self as *const _ as *mut EventLoopHandle,
            idata,
        ));

        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_fd,
                self.ptr,
                fd,
                interest.bits(),
                ::event_sources::event_source_fd_dispatcher::<ID>,
                &*data as *const _ as *mut c_void
            )
        };
        if ret.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(::event_sources::make_fd_event_source(ret, data))
        }
    }

    /// Add a timer event source to this event loop
    ///
    /// It is a countdown, which can be reset using the struct
    /// returned by this function. When the countdown reaches 0,
    /// the registered handler is called in the dispatching of
    /// this event loop.
    pub fn add_timer_event_source<ID>(&mut self, implementation: ::event_sources::TimerEventSourceImpl<ID>,
                                      idata: ID)
                                      -> IoResult<::event_sources::TimerEventSource<ID>>
    where
        ID: 'static,
    {
        let data = Box::new((
            implementation,
            self as *const _ as *mut EventLoopHandle,
            idata,
        ));

        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_timer,
                self.ptr,
                ::event_sources::event_source_timer_dispatcher::<ID>,
                &*data as *const _ as *mut c_void
            )
        };
        if ret.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(::event_sources::make_timer_event_source(ret, data))
        }
    }

    /// Add a signal event source to this event loop
    ///
    /// This will listen for a given unix signal (by setting up
    /// a signalfd for it) and call the registered handler whenever
    /// the program receives this signal. Calls are made during the
    /// dispatching of this event loop.
    pub fn add_signal_event_source<ID>(&mut self,
                                       implementation: ::event_sources::SignalEventSourceImpl<ID>,
                                       idata: ID, signal: ::nix::sys::signal::Signal)
                                       -> IoResult<::event_sources::SignalEventSource<ID>>
    where
        ID: 'static,
    {
        let data = Box::new((
            implementation,
            self as *const _ as *mut EventLoopHandle,
            idata,
        ));

        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_signal,
                self.ptr,
                signal as c_int,
                ::event_sources::event_source_signal_dispatcher::<ID>,
                &*data as *const _ as *mut c_void
            )
        };
        if ret.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(::event_sources::make_signal_event_source(ret, data))
        }
    }

    /// Add an idle event source to this event loop
    ///
    /// This is a kind of "defer this computation for when there is nothing else to do".
    ///
    /// The provided implementation callback will be called when the event loop has finished
    /// processing all the pending I/O. This callback will be fired exactly once the first
    /// time this condition is met.
    ///
    /// You can cancel it using the returned `IdleEventSource`.
    pub fn add_idle_event_source<ID>(&mut self, implementation: ::event_sources::IdleEventSourceImpl<ID>,
                                     idata: ID)
                                     -> ::event_sources::IdleEventSource<ID>
    where
        ID: 'static,
    {
        let data = Rc::new(RefCell::new((
            implementation,
            self as *const _ as *mut EventLoopHandle,
            idata,
            false,
        )));

        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_idle,
                self.ptr,
                ::event_sources::event_source_idle_dispatcher::<ID>,
                Rc::into_raw(data.clone()) as *mut _
            )
        };
        ::event_sources::make_idle_event_source(ret, data)
    }

    /// Register a global object to the display.
    ///
    /// Specify the version of the interface to advertize, as well as the callback that will
    /// receive requests to create an object.
    ///
    /// This uses an "implementation data" mechanism similar to regular wayland objects.
    ///
    /// Panics:
    ///
    /// - if the event loop is not associated to a display
    pub fn register_global<R: Resource, ID>(&mut self, version: i32, callback: GlobalCallback<R, ID>,
                                            idata: ID)
                                            -> Global<R, ID> {
        let display = self.display
            .expect("Globals can only be registered on an event loop associated with a display.");

        let data = Box::new((
            callback,
            self as *const _ as *mut EventLoopHandle,
            idata,
        ));

        let ptr = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_global_create,
                display,
                R::interface_ptr(),
                version,
                &*data as *const (GlobalCallback<R, ID>, *mut EventLoopHandle, ID) as *mut _,
                global_bind::<R, ID>
            )
        };

        Global {
            ptr: ptr,
            data: Box::into_raw(data),
        }
    }
}

/// Checks if a resource is registered with a given implementation on an event loop
///
/// Returns `false` if the resource is dead, even if it was registered with
/// this implementation while alive.
pub fn resource_is_registered<R, ID>(resource: &R, implementation: &R::Implementation) -> bool
where
    R: Resource + Implementable<ID>,
{
    if resource.status() != ::Liveness::Alive {
        return false;
    }
    let resource_data = unsafe {
        &*(ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_get_user_data,
            resource.ptr()
        ) as *mut ResourceUserData<R>)
    };
    if resource_data.0.is_null() {
        return false;
    }
    ((&*resource_data).1)
        .as_ref()
        .and_then(|v| v.downcast_ref::<(R::Implementation, ID)>())
        .map(|t| &t.0 == implementation)
        .unwrap_or(false)
}

pub unsafe fn create_event_loop(ptr: *mut wl_event_loop, display: Option<*mut wl_display>) -> EventLoop {
    EventLoop {
        handle: Box::new(EventLoopHandle {
            state: State::new(),
            keep_going: false,
            ptr: ptr,
            display: display,
        }),
    }
}

pub struct EventLoop {
    handle: Box<EventLoopHandle>,
}

/// Callback function called when a global is instanciated by a client
///
/// Arguments are:
///
/// - handle to the eventloop
/// - implementation data you provided to `register_global`
/// - client that instanciated the global
/// - the newly instanciated global
pub type GlobalCallback<R, ID> = fn(&mut EventLoopHandle, &mut ID, &Client, R);

impl EventLoop {
    /// Create a new EventLoop
    ///
    /// It is not associated to a wayland socket, and can be used for other
    /// event sources.
    pub fn new() -> EventLoop {
        unsafe {
            let ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_create,);
            create_event_loop(ptr, None)
        }
    }

    /// Dispatch pending requests to their respective handlers
    ///
    /// If no request is pending, will block at most `timeout` ms if specified,
    /// or indefinitely if `timeout` is `None`.
    ///
    /// Returns the number of requests dispatched or an error.
    pub fn dispatch(&mut self, timeout: Option<u32>) -> IoResult<u32> {
        use std::i32;
        let timeout = match timeout {
            None => -1,
            Some(v) if v >= (i32::MAX as u32) => i32::MAX,
            Some(v) => (v as i32),
        };
        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_dispatch,
                self.ptr,
                timeout
            )
        };
        if ret >= 0 {
            Ok(ret as u32)
        } else {
            Err(IoError::last_os_error())
        }
    }

    /// Runs the event loop
    ///
    /// This method will call repetitively the dispatch method,
    /// until one of the handlers call the `stop_loop` method
    /// on the `EventLoopHandle`.
    ///
    /// If this event loop is attached to a display, it will also
    /// flush the events to the clients between two calls to
    /// `dispatch()`.
    ///
    /// Note that this method will block indefinitely on waiting events,
    /// as such, if you need to avoid a complete block even if no events
    /// are received, you should use the `dispatch()` method instead and
    /// set a timeout.
    pub fn run(&mut self) -> IoResult<()> {
        self.handle.keep_going = true;
        loop {
            if let Some(display) = self.handle.display {
                unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_flush_clients, display) };
            }
            self.dispatch(None)?;
            if !self.handle.keep_going {
                return Ok(());
            }
        }
    }
}

impl Deref for EventLoop {
    type Target = EventLoopHandle;
    fn deref(&self) -> &EventLoopHandle {
        &*self.handle
    }
}

impl DerefMut for EventLoop {
    fn deref_mut(&mut self) -> &mut EventLoopHandle {
        &mut *self.handle
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        if self.handle.display.is_none() {
            // only destroy the event_loop if it's not the one
            // from the display
            unsafe {
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_destroy, self.ptr);
            }
        }
    }
}

unsafe extern "C" fn dispatch_func<R, ID>(_impl: *const c_void, resource: *mut c_void, opcode: u32,
                                          _msg: *const wl_message, args: *const wl_argument)
                                          -> c_int
where
    R: Resource + Implementable<ID>,
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
        // This cast from *const to *mut is legit because we enforce that a Handler
        // can only be assigned to a single EventQueue.
        // (this is actually the whole point of the design of this lib)
        let resource = R::from_ptr_initialized(resource as *mut wl_resource);
        let client = Client::from_ptr(ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_get_client,
            resource.ptr()
        ));
        resource.__dispatch_msg(&client, opcode, args)
    });
    match ret {
        Ok(Ok(())) => return 0, // all went well
        Ok(Err(())) => {
            // an unknown opcode was dispatched, this is not normal
            let _ = write!(
                ::std::io::stderr(),
                "[wayland-server error] Attempted to dispatch unknown opcode {} for {}, aborting.",
                opcode,
                R::interface_name()
            );
            ::libc::abort();
        }
        Err(_) => {
            // a panic occured
            let _ = write!(
                ::std::io::stderr(),
                "[wayland-server error] A handler for {} panicked, aborting.",
                R::interface_name()
            );
            ::libc::abort();
        }
    }
}

unsafe extern "C" fn global_bind<R: Resource, ID>(client: *mut wl_client, data: *mut c_void, version: u32,
                                                  id: u32) {
    // safety of this function is the same as dispatch_func
    let ret = ::std::panic::catch_unwind(move || {
        let data = &mut *(data as *mut (GlobalCallback<R, ID>, *mut EventLoopHandle, ID));
        let cb = data.0;
        let evqhandle = &mut *data.1;
        let idata = &mut data.2;
        let client = Client::from_ptr(client);
        let ptr = ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_create,
            client.ptr(),
            R::interface_ptr(),
            version as i32, // wayland already checks the validity of the version
            id
        );
        let resource = R::from_ptr_new(ptr as *mut wl_resource);
        cb(evqhandle, idata, &client, resource)
    });
    match ret {
        Ok(()) => (), // all went well
        Err(_) => {
            // a panic occured
            let _ = write!(
                ::std::io::stderr(),
                "[wayland-server error] A global handler for {} panicked, aborting.",
                R::interface_name()
            );
            ::libc::abort();
        }
    }
}

unsafe extern "C" fn resource_destroy<R: Resource>(resource: *mut wl_resource) {
    let resource = R::from_ptr_initialized(resource as *mut wl_resource);
    if resource.status() == ::Liveness::Alive {
        // mark the resource as dead
        let data = &mut *(ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_get_user_data,
            resource.ptr()
        ) as *mut ResourceUserData<R>);
        (data.2)
            .0
            .store(false, ::std::sync::atomic::Ordering::SeqCst);
        if let Some(destructor) = data.3 {
            destructor(&resource)
        }
    }
}
