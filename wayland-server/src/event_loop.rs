use std::any::Any;
use std::io::{Result as IoResult, Error as IoError};
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_void, c_int};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr};

use wayland_sys::common::{wl_message, wl_argument};
use wayland_sys::server::*;
use {Resource, Handler, Client};

type ResourceUserData = (*mut EventLoopHandle, Arc<(AtomicBool, AtomicPtr<()>)>);

/// A handle to a global object
///
/// This is given to you when you register a global to the event loop.
///
/// This handle allows you do destroy the global when needed.
///
/// If you know you will never destroy this global, you can let this
/// handle go out of scope.
pub struct Global {
    ptr: *mut wl_global,
    _data: Box<(*mut c_void, *mut EventLoopHandle)>
}

unsafe impl Send for Global {}

impl Global {
    /// Destroy the associated global object.
    pub fn destroy(self) {
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_global_destroy, self.ptr);
        }
    }
}

/// Trait to handle a global object.
pub trait GlobalHandler<R: Resource> {
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
    fn bind(&mut self, evqh: &mut EventLoopHandle, client: &Client, global: R);
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
    handlers: Vec<Box<Any + Send>>,
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
    pub fn register<R: Resource, H: Handler<R> + Any + Send + 'static>(&mut self, resource: &R, handler_id: usize) {
        let h = self.handlers[handler_id].downcast_ref::<H>()
                    .expect("Handler type do not match.");
        unsafe {
            let data: *mut ResourceUserData = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_get_user_data,
                resource.ptr()
            ) as *mut _;
            (&mut *data).0 = self as *const _  as *mut _;
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_set_dispatcher,
                resource.ptr(),
                dispatch_func::<R,H>,
                h as *const _ as *const c_void,
                data as *mut c_void,
                None
            );
        }
    }

    /// Register a resource to a handler of this event loop with a destructor
    ///
    /// The H type must be provided and match the type of the targetted Handler, or
    /// it will panic.
    ///
    /// The D type is the one whose `Destroy<R>` impl will be used as destructor.
    ///
    /// This overwrites any precedently set Handler and destructor for this resource.
    pub fn register_with_destructor<R, H, D>(&mut self, resource: &R, handler_id: usize)
        where R: Resource,
              H: Handler<R> + Any + Send + 'static,
              D: Destroy<R> + 'static
    {
        let h = self.handlers[handler_id].downcast_ref::<H>()
                    .expect("Handler type do not match.");
        unsafe {
            let data: *mut ResourceUserData = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_get_user_data,
                resource.ptr()
            ) as *mut _;
            (&mut *data).0 = self as *const _  as *mut _;
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_set_dispatcher,
                resource.ptr(),
                dispatch_func::<R,H>,
                h as *const _ as *const c_void,
                data as *mut c_void,
                Some(resource_destroy::<R, D>)
            );
        }
    }

    /// Insert a new handler to this EventLoop
    ///
    /// Returns the index of this handler in the internal array, needed register
    /// proxies to it.
    pub fn add_handler<H: Any + Send + 'static>(&mut self, handler: H) -> usize {
        self.handlers.push(Box::new(handler) as Box<Any + Send>);
        self.handlers.len() - 1
    }

    /// Insert a new handler with init
    ///
    /// Allows you to insert handlers that require some interaction with the
    /// event loop in their initialization, like registering some objects to it.
    ///
    /// The handler must implement the `Init` trait, and its init method will
    /// be called after its insertion.
    pub fn add_handler_with_init<H: Init + Any + Send + 'static>(&mut self, handler: H) -> usize
    {
        let mut box_ = Box::new(handler);
        // this little juggling is to avoid the double-borrow, which is actually safe,
        // as handlers cannot be mutably accessed outside of an event-dispatch,
        // and this new handler cannot receive any events before the return
        // of this function
        let h = &mut *box_ as *mut H;
        self.handlers.push(box_ as Box<Any + Send>);
        let index = self.handlers.len() - 1;
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
}

/// Checks if a resource is registered with a given handler on an event loop
///
/// The H type must be provided and match the type of the targetted Handler, or
/// it will panic.
pub fn resource_is_registered<R, H>(resource: &R, handler_id: usize) -> bool
    where R: Resource,
          H: Handler<R> + Any + Send + 'static
{
    let resource_data = unsafe { &*(ffi_dispatch!(
        WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource.ptr()
    ) as *mut ResourceUserData) };
    if resource_data.0.is_null() {
        return false;
    }
    let evlh = unsafe { &*(resource_data.0) };
    let h = evlh.handlers[handler_id].downcast_ref::<H>()
                .expect("Handler type do not match.");
    let ret = unsafe {
        ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_resource_instance_of,
            resource.ptr(),
            R::interface_ptr(),
            h as *const _ as *const c_void
        )
    };
    ret == 1
}

/// Guard to access internal state of an event loop
///
/// This guard allows you to get references to the handlers you
/// previously stored inside an event loop.
///
/// It borrows the event loop, so no event dispatching is possible
/// as long as the guard is in scope, for safety reasons.
pub struct StateGuard<'evq> {
    evq: &'evq mut EventLoop
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

pub unsafe fn create_event_loop(ptr: *mut wl_event_loop, display: Option<*mut wl_display>) -> EventLoop {
    EventLoop {
        ptr: ptr,
        display: display,
        handle: Box::new(EventLoopHandle { handlers: Vec::new(), keep_going: false })
    }
}

pub struct EventLoop {
    ptr: *mut wl_event_loop,
    display: Option<*mut wl_display>,
    handle: Box<EventLoopHandle>
}

impl EventLoop {
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
            Some(v) => (v as i32)
        };
        let ret = unsafe { ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_event_loop_dispatch,
            self.ptr,
            timeout
        )};
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
    pub fn run(&mut self) -> IoResult<()> {
        self.handle.keep_going = true;
        loop {
            if let Some(display) = self.display {
                unsafe { ffi_dispatch!(
                    WAYLAND_SERVER_HANDLE,
                    wl_display_flush_clients,
                    display
                )};
            }
            try!(self.dispatch(None));
            if !self.handle.keep_going {
                return Ok(())
            }
        }
    }

    /// Register a global object to the display.
    ///
    /// Specify the version of the interface to advertize, as well as the handler that will
    /// receive requests to create an object.
    ///
    /// The handler must implement the appropriate `GlobalHandler<R>` trait.
    ///
    /// Panics:
    ///
    /// - if the event loop is not associated to a display
    /// - if the provided `H` type does not match the actual type of the handler
    pub fn register_global<R: Resource, H: GlobalHandler<R> + Any + 'static>(&mut self, handler_id: usize, version: i32) -> Global {
        let h = self.handle.handlers[handler_id].downcast_ref::<H>()
                    .expect("Handler type do not match.");
        let display = self.display.expect("Globals can only be registered on an event loop associated with a display.");

        let data = Box::new((h as *const _ as *mut c_void, &*self.handle as *const _ as *mut EventLoopHandle));

        let ptr = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_global_create,
                display,
                R::interface_ptr(),
                version,
                &*data as *const (*mut c_void, *mut EventLoopHandle) as *mut _,
                global_bind::<R,H>
            )
        };

        Global {
            ptr: ptr,
            _data: data
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
}

unsafe impl Send for EventLoop { }

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

unsafe extern "C" fn dispatch_func<R: Resource, H: Handler<R>>(
    handler: *const c_void,
    resource: *mut c_void,
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
        let resource = R::from_ptr_initialized(resource as *mut wl_resource);
        let data = &mut *(ffi_dispatch!(
            WAYLAND_SERVER_HANDLE, wl_resource_get_user_data, resource.ptr()
        ) as *mut ResourceUserData);
        let evqhandle = &mut *data.0;
        let client = Client::from_ptr(ffi_dispatch!(
            WAYLAND_SERVER_HANDLE, wl_resource_get_client, resource.ptr()
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

unsafe extern "C" fn global_bind<R: Resource, H: GlobalHandler<R>>(
    client: *mut wl_client,
    data: *mut c_void,
    version: u32,
    id: u32
) {
    // safety of this function is the same as dispatch_fund
    let ret = ::std::panic::catch_unwind(move || {
        let data = &*(data as *const (*mut H, *mut EventLoopHandle));
        let handler = &mut *data.0;
        let evqhandle = &mut *data.1;
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
        handler.bind(evqhandle, &client, resource)
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

// TODO : figure out how it is used exactly
unsafe extern "C" fn resource_destroy<R: Resource, D: Destroy<R>>(resource: *mut wl_resource) {
    let resource = R::from_ptr_initialized(resource as *mut wl_resource);
    D::destroy(&resource);
}

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
#[macro_export]
macro_rules! declare_handler(
    ($handler_struct: ty, $handler_trait: path, $handled_type: ty) => {
        unsafe impl $crate::Handler<$handled_type> for $handler_struct {
            unsafe fn message(&mut self, evq: &mut $crate::EventLoopHandle, client: &$crate::Client, proxy: &$handled_type, opcode: u32, args: *const $crate::sys::wl_argument) -> Result<(),()> {
                <$handler_struct as $handler_trait>::__message(self, evq, client, proxy, opcode, args)
            }
        }
    }
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
#[macro_export]
macro_rules! declare_delegating_handler(
    ($handler_struct: ty, $($handler_field: ident).+ , $handler_trait: path, $handled_type: ty) => {
        unsafe impl $crate::Handler<$handled_type> for $handler_struct {
            unsafe fn message(&mut self, evq: &mut $crate::EventLoopHandle, client: &$crate::Client, proxy: &$handled_type, opcode: u32, args: *const $crate::sys::wl_argument) -> Result<(),()> {
                <$handler_trait>::__message(&mut self.$($handler_field).+, evq, client, proxy, opcode, args)
            }
        }
    }
);
