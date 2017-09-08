use {Client, Handler, Resource};
use std::any::Any;
use std::io::{Error as IoError, Result as IoResult};
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_int, c_void};
use std::os::unix::io::RawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr};
use wayland_sys::RUST_MANAGED;

use wayland_sys::common::{wl_argument, wl_message};
use wayland_sys::server::*;

type ResourceUserData = (*mut EventLoopHandle, *mut c_void, Arc<(AtomicBool, AtomicPtr<()>)>);

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
pub struct Global<U: Send + 'static> {
    ptr: *mut wl_global,
    data: *mut (*mut c_void, *mut EventLoopHandle, *mut U),
}

unsafe impl<U: Send + 'static> Send for Global<U> {}

impl<U: Send + 'static> Global<U> {
    /// Destroy the associated global object.
    pub fn destroy(self) {
        unsafe {
            // destroy the global
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_destroy, self.ptr);
            // free the user data
            let data = Box::from_raw(self.data);
            let user_data = Box::from_raw(data.2);
            drop(data);
            drop(user_data);
        }
    }
}

/// Trait to handle a global object.
pub trait GlobalHandler<R: Resource, U: Send> {
    /// Request to bind a global
    ///
    /// This method is called each time a client binds this global object from
    /// the registry.
    ///
    /// The global is instantiated as a `Resource` and provided to the callback,
    /// do whatever you need with it.
    ///
    /// Letting it out of scope will *not* destroy the resource, and you'll still
    /// receive its events (as long as you've registered an appropriate handler).
    fn bind(&mut self, evqh: &mut EventLoopHandle, client: &Client, global: R, user_data: &mut U);
}

/// A trait to initialize handlers after they've been inserted in an event queue
///
/// Works with the `add_handler_with_init` method of `EventQueueHandle`.
pub trait Init {
    /// Init the handler
    ///
    /// `index` is the current index of the handler in the event queue (you can
    /// use it to register objects to it)
    fn init(&mut self, evqh: &mut EventLoopHandle, index: usize);
}

/// A trait to handle destruction of ressources.
///
/// This is usefull if you need to deallocate user data for example.
///
/// This is a trait with a single static method rather (than a freestanding function)
/// in order to internally profit of static dispatch.
pub trait Destroy<R: Resource> {
    /// Destructor of a resource
    ///
    /// This function is called right before a resource is destroyed, if it has
    /// been assigned.
    ///
    /// To assign a destructor to a resource, see `EventLoopHandle::register_with_destructor`.
    fn destroy(resource: &R);
}

/// Handle to an event loop
///
/// This handle gives you access to methods on an event loop
/// that are safe to do from within a callback.
///
/// They are also available on an `EventLoop` object via `Deref`.
pub struct EventLoopHandle {
    handlers: Vec<Option<Box<Any + Send>>>,
    keep_going: bool,
}

impl EventLoopHandle {
    /// Register a resource to a handler of this event loop.
    ///
    /// The H type must be provided and match the type of the targetted Handler, or
    /// it will panic.
    ///
    /// This overwrites any precedently set Handler for this resource and removes its destructor
    /// if any.
    ///
    /// Returns an error and does nothing if this resource is dead or already managed by
    /// something else than this library.
    pub fn register<R, H>(&mut self, resource: &R, handler_id: usize) -> RegisterStatus
    where
        R: Resource,
        H: Handler<R> + Any + Send + 'static,
    {
        self.register_with_destructor::<R, H, NoopDestroy>(resource, handler_id)
    }

    /// Register a resource to a handler of this event loop with a destructor
    ///
    /// The H type must be provided and match the type of the targetted Handler, or
    /// it will panic.
    ///
    /// The D type is the one whose `Destroy<R>` impl will be used as destructor.
    ///
    /// This overwrites any precedently set Handler and destructor for this resource.
    ///
    /// Returns an error and does nothing if this resource is dead or already managed by
    /// something else than this library.
    pub fn register_with_destructor<R, H, D>(&mut self, resource: &R, handler_id: usize) -> RegisterStatus
    where
        R: Resource,
        H: Handler<R> + Any + Send + 'static,
        D: Destroy<R> + 'static,
    {
        let h = self.handlers[handler_id]
            .as_ref()
            .expect("Handler has already been removed")
            .downcast_ref::<H>()
            .expect("Handler type do not match.");

        match resource.status() {
            ::Liveness::Dead => return RegisterStatus::Dead,
            ::Liveness::Unmanaged => return RegisterStatus::Unmanaged,
            ::Liveness::Alive => { /* ok, we can continue */ }
        }

        unsafe {
            let data: *mut ResourceUserData = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_get_user_data,
                resource.ptr()
            ) as *mut _;
            // This cast from *const to *mut is legit because we enforce that a Handler
            // can only be assigned to a single EventQueue.
            // (this is actually the whole point of the design of this lib)
            (&mut *data).0 = self as *const _ as *mut _;
            (&mut *data).1 = h as *const _ as *mut c_void;
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_set_dispatcher,
                resource.ptr(),
                dispatch_func::<R, H>,
                &RUST_MANAGED as *const _ as *const _,
                data as *mut c_void,
                Some(resource_destroy::<R, D>)
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

    /// Insert a new handler to this EventLoop
    ///
    /// Returns the index of this handler in the internal array, needed register
    /// proxies to it.
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

    /// Stop looping
    ///
    /// If the event loop this handle belongs to is currently running its `run()`
    /// method, it'll stop and return as soon as the current dispatching session ends.
    pub fn stop_loop(&mut self) {
        self.keep_going = false;
    }

    /// Remove a handler previously inserted in this event loop and returns it.
    ///
    /// Panics if the requested type does not match the type of the stored handler
    /// or if the specified index was already removed.
    ///
    /// **Unsafety** This function is unsafe because removing a handler while some wayland
    /// objects or event sources are still registered to it can lead to access to freed memory.
    /// Also, the index of this handler will be reused at next handler insertion.
    pub unsafe fn remove_handler<H: Any + Send + 'static>(&mut self, idx: usize) -> H {
        let is_type = self.handlers[idx]
            .as_ref()
            .expect("Handler has already been removed.")
            .is::<H>();
        assert!(is_type, "Handler type do not match.");
        *(self.handlers[idx].take().unwrap().downcast().unwrap())
    }
}

/// Checks if a resource is registered with a given handler on an event loop
///
/// The H type must be provided and match the type of the targetted Handler, or
/// it will panic.
///
/// Returns `false` if the resource is dead, even if it was registered to this
/// handler while alive.
pub fn resource_is_registered<R, H>(resource: &R, handler_id: usize) -> bool
where
    R: Resource,
    H: Handler<R> + Any + Send + 'static,
{
    if resource.status() != ::Liveness::Alive {
        return false;
    }
    let resource_data = unsafe {
        &*(ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_get_user_data,
            resource.ptr()
        ) as *mut ResourceUserData)
    };
    if resource_data.0.is_null() {
        return false;
    }
    let evlh = unsafe { &*(resource_data.0) };
    let h = evlh.handlers[handler_id]
        .as_ref()
        .expect("Handler has already been removed.")
        .downcast_ref::<H>()
        .expect("Handler type do not match.");
    (&*resource_data).1 == h as *const _ as *mut c_void
}

/// Guard to access internal state of an event loop
///
/// This guard allows you to get references to the handlers you
/// previously stored inside an event loop.
///
/// It borrows the event loop, so no event dispatching is possible
/// as long as the guard is in scope, for safety reasons.
pub struct StateGuard<'evq> {
    evq: &'evq mut EventLoop,
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
            .expect("Handler has already been removed.")
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
            .expect("Handler has already been removed.")
            .downcast_mut::<H>()
            .expect("Handler type do not match.")
    }
}

pub unsafe fn create_event_loop(ptr: *mut wl_event_loop, display: Option<*mut wl_display>) -> EventLoop {
    EventLoop {
        ptr: ptr,
        display: display,
        handle: Box::new(EventLoopHandle {
            handlers: Vec::new(),
            keep_going: false,
        }),
    }
}

pub struct EventLoop {
    ptr: *mut wl_event_loop,
    display: Option<*mut wl_display>,
    handle: Box<EventLoopHandle>,
}

impl EventLoop {
    /// Create a new EventLoop
    ///
    /// It is not associated to a wayland socket, and can be used for other
    /// event sources.
    pub fn new() -> EventLoop {
        unsafe {
            let ptr =
                ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_create,
            );
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
            if let Some(display) = self.display {
                unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_flush_clients, display) };
            }
            self.dispatch(None)?;
            if !self.handle.keep_going {
                return Ok(());
            }
        }
    }

    /// Register a global object to the display.
    ///
    /// Specify the version of the interface to advertize, as well as the handler that will
    /// receive requests to create an object.
    ///
    /// The `user_data` is a value that will be provided as argument to `Global::Bind`. This allows
    /// you to store global-specific data in case you are willing to have several globals using the
    /// same handler. This way, your handler can differentiate which of these global was
    /// instanciated. If you have no use for it, just use `()`.
    ///
    /// The handler must implement the appropriate `GlobalHandler<R>` trait.
    ///
    /// Panics:
    ///
    /// - if the event loop is not associated to a display
    /// - if the provided `H` type does not match the actual type of the handler
    pub fn register_global<R, U, H>(&mut self, handler_id: usize, version: i32, user_data: U) -> Global<U>
    where
        R: Resource,
        U: Send + 'static,
        H: GlobalHandler<R, U> + Any + 'static,
    {
        let h = self.handle.handlers[handler_id]
            .as_ref()
            .expect("Handler has already been removed.")
            .downcast_ref::<H>()
            .expect("Handler type do not match.");
        let display = self.display.expect(
            "Globals can only be registered on an event loop associated with a display.",
        );

        let data = Box::new((
            h as *const _ as *mut c_void,
            &*self.handle as *const _ as *mut EventLoopHandle,
            Box::into_raw(Box::new(user_data)),
        ));

        let ptr = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_global_create,
                display,
                R::interface_ptr(),
                version,
                &*data as *const (*mut c_void, *mut EventLoopHandle, *mut U) as *mut _,
                global_bind::<R, U, H>
            )
        };

        Global {
            ptr: ptr,
            data: Box::into_raw(data),
        }
    }

    /// Get an handle to the internal state
    ///
    /// The returned guard object allows you to get references
    /// to the handler objects you previously inserted in this
    /// event loop.
    pub fn state(&mut self) -> StateGuard {
        StateGuard { evq: self }
    }

    /// Add a File Descriptor event source to this event loop
    ///
    /// The interest in read/write capability for this FD must be provided
    /// (and can be changed afterwards using the returned object), and the
    /// associated handler will be called whenever these capabilities are
    /// satisfied, during the dispatching of this event loop.
    pub fn add_fd_event_source<H>(&mut self, fd: RawFd, handler_id: usize,
                                  interest: ::event_sources::FdInterest)
                                  -> IoResult<::event_sources::FdEventSource>
    where
        H: ::event_sources::FdEventSourceHandler + 'static,
    {
        let h = self.handlers[handler_id]
            .as_ref()
            .expect("Handler has already been removed.")
            .downcast_ref::<H>()
            .expect("Handler type do not match.");
        let data = Box::new((
            h as *const _ as *mut c_void,
            &*self.handle as *const _ as *mut EventLoopHandle,
        ));

        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_fd,
                self.ptr,
                fd,
                interest.bits(),
                ::event_sources::event_source_fd_dispatcher::<H>,
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
    pub fn add_timer_event_source<H>(&mut self, handler_id: usize)
                                     -> IoResult<::event_sources::TimerEventSource>
    where
        H: ::event_sources::TimerEventSourceHandler + 'static,
    {
        let h = self.handlers[handler_id]
            .as_ref()
            .expect("Handler has already been removed.")
            .downcast_ref::<H>()
            .expect("Handler type do not match.");
        let data = Box::new((
            h as *const _ as *mut c_void,
            &*self.handle as *const _ as *mut EventLoopHandle,
        ));

        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_timer,
                self.ptr,
                ::event_sources::event_source_timer_dispatcher::<H>,
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
    pub fn add_signal_event_source<H>(&mut self, signal: ::nix::sys::signal::Signal, handler_id: usize)
                                      -> IoResult<::event_sources::SignalEventSource>
    where
        H: ::event_sources::SignalEventSourceHandler + 'static,
    {
        let h = self.handlers[handler_id]
            .as_ref()
            .expect("Handler has already been removed.")
            .downcast_ref::<H>()
            .expect("Handler type do not match.");
        let data = Box::new((
            h as *const _ as *mut c_void,
            &*self.handle as *const _ as *mut EventLoopHandle,
        ));

        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_signal,
                self.ptr,
                signal as c_int,
                ::event_sources::event_source_signal_dispatcher::<H>,
                &*data as *const _ as *mut c_void
            )
        };
        if ret.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(::event_sources::make_signal_event_source(ret, data))
        }
    }
}

unsafe impl Send for EventLoop {}

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
        if self.display.is_none() {
            // only destroy the event_loop if it's not the one
            // from the display
            unsafe {
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_destroy, self.ptr);
            }
        }
    }
}

unsafe extern "C" fn dispatch_func<R: Resource, H: Handler<R>>(_impl: *const c_void, resource: *mut c_void,
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
        // This cast from *const to *mut is legit because we enforce that a Handler
        // can only be assigned to a single EventQueue.
        // (this is actually the whole point of the design of this lib)
        let resource = R::from_ptr_initialized(resource as *mut wl_resource);
        let data = &mut *(ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_get_user_data,
            resource.ptr()
        ) as *mut ResourceUserData);
        let evqhandle = &mut *data.0;
        let handler = &mut *(data.1 as *mut H);
        let client = Client::from_ptr(ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_get_client,
            resource.ptr()
        ));
        handler.message(evqhandle, &client, &resource, opcode, args)
    });
    match ret {
        Ok(Ok(())) => return 0,   // all went well
        Ok(Err(())) => {
            // an unknown opcode was dispatched, this is not normal
            let _ = write!(
                ::std::io::stderr(),
                "[wayland-server error] Attempted to dispatch unknown opcode {} for {}, aborting.",
                opcode, R::interface_name()
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

unsafe extern "C" fn global_bind<R, U, H>(client: *mut wl_client, data: *mut c_void, version: u32, id: u32)
where
    R: Resource,
    U: Send + 'static,
    H: GlobalHandler<R, U>,
{
    // safety of this function is the same as dispatch_func
    let ret = ::std::panic::catch_unwind(move || {
        let data = &*(data as *const (*mut H, *mut EventLoopHandle, *mut U));
        let handler = &mut *data.0;
        let evqhandle = &mut *data.1;
        let user_data = &mut *data.2;
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
        handler.bind(evqhandle, &client, resource, user_data)
    });
    match ret {
        Ok(()) => (),   // all went well
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

struct NoopDestroy;

impl<R: Resource> Destroy<R> for NoopDestroy {
    fn destroy(_: &R) {}
}

unsafe extern "C" fn resource_destroy<R: Resource, D: Destroy<R>>(resource: *mut wl_resource) {
    let resource = R::from_ptr_initialized(resource as *mut wl_resource);
    if resource.status() == ::Liveness::Alive {
        // mark the resource as dead
        let data = &mut *(ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_get_user_data,
            resource.ptr()
        ) as *mut ResourceUserData);
        (data.2).0.store(
            false,
            ::std::sync::atomic::Ordering::SeqCst,
        );
    }
    D::destroy(&resource);
}

/// Synonym of the declare_handler! macro.
///
/// This macro with a more distinctive name can be used for projects
/// that need to use both client-side and server-side macros.
#[macro_export]
macro_rules! server_declare_handler(
    ($handler_struct: ident <$($tyarg:ident : [$($trait: ident $(<$($traitarg: ty),*>)*),*]),*>, $handler_trait: path, $handled_type: ty) => {
        unsafe impl<$($tyarg : $($trait $(<$($traitarg),*>)* +)* 'static),*> $crate::Handler<$handled_type> for $handler_struct<$($tyarg),*> {
            unsafe fn message(&mut self,
                              evq: &mut $crate::EventLoopHandle,
                              client: &$crate::Client,
                              proxy: &$handled_type,
                              opcode: u32,
                              args: *const $crate::sys::wl_argument
                             ) -> ::std::result::Result<(),()> {
                <$handler_trait>::__message(self, evq, client, proxy, opcode, args)
            }
        }
    };
    ($handler_struct: ident, $handler_trait: path, $handled_type: ty) => {
        unsafe impl $crate::Handler<$handled_type> for $handler_struct {
            unsafe fn message(&mut self,
                              evq: &mut $crate::EventLoopHandle,
                              client: &$crate::Client,
                              proxy: &$handled_type,
                              opcode: u32,
                              args: *const $crate::sys::wl_argument
                             ) -> ::std::result::Result<(),()> {
                <$handler_trait>::__message(self, evq, client, proxy, opcode, args)
            }
        }
    };
);

/// Registers a handler type so it can be used in event loops
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
    ($handler_struct: ident <$($tyarg:ident : [$($trait: ident $(<$($traitarg: ty),*>)*),*]),*>, $handler_trait: path, $handled_type: ty) => {
        server_declare_handler!($handler_struct<$($tyarg: [$($trait $(<$($traitarg),*>)*),*]),*>, $handler_trait, $handled_type);
    };
    ($handler_struct: ident, $handler_trait: path, $handled_type: ty) => {
        server_declare_handler!($handler_struct, $handler_trait, $handled_type);
    };
);

/// Synonym of the declare_delegating_handler! macro.
///
/// This macro with a more distinctive name can be used for projects
/// that need to use both client-side and server-side macros.
#[macro_export]
macro_rules! server_declare_delegating_handler(
    ($handler_struct: ident <$($tyarg:ident : [$($trait: ident $(<$($traitarg: ty),*>)*),*]),*>, $($handler_field: ident).+ , $handler_trait: path, $handled_type: ty) => {
        unsafe impl<$($tyarg : $($trait $(<$($traitarg),*>)* +)* 'static),*> $crate::Handler<$handled_type> for $handler_struct<$($tyarg),*> {
            unsafe fn message(&mut self,
                              evq: &mut $crate::EventLoopHandle,
                              client: &$crate::Client,
                              proxy: &$handled_type,
                              opcode: u32,
                              args: *const $crate::sys::wl_argument
                             ) -> ::std::result::Result<(),()> {
                <$handler_trait>::__message(&mut self.$($handler_field).+, evq, client, proxy, opcode, args)
            }
        }
    };
    ($handler_struct: ident, $($handler_field: ident).+ , $handler_trait: path, $handled_type: ty) => {
        unsafe impl $crate::Handler<$handled_type> for $handler_struct {
            unsafe fn message(&mut self,
                              evq: &mut $crate::EventLoopHandle,
                              client: &$crate::Client,
                              proxy: &$handled_type,
                              opcode: u32,
                              args: *const $crate::sys::wl_argument
                             ) -> ::std::result::Result<(),()> {
                <$handler_trait>::__message(&mut self.$($handler_field).+, evq, client, proxy, opcode, args)
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
        server_declare_delegating_handler!($handler_struct<$($tyarg: [$($trait $(<$($traitarg),*>)*),*]),*>, $($handler_field).+, $handler_trait, $handled_type);
    };
    ($handler_struct: ident, $($handler_field: ident).+ , $handler_trait: path, $handled_type: ty) => {
        server_declare_delegating_handler!($handler_struct, $($handler_field).+, $handler_trait, $handled_type);
    };
);
