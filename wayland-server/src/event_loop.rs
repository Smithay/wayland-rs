use std::any::Any;
use std::io::{Result as IoResult, Error as IoError};
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_void, c_int};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use wayland_sys::common::{wl_message, wl_argument};
use wayland_sys::server::*;
use {Resource, Handler, Client};

type ResourceUserData = (*mut EventLoopHandle, Arc<AtomicBool>);

pub struct Global {
    ptr: *mut wl_global,
    _data: Box<(*mut c_void, *mut EventLoopHandle)>
}

impl Global {
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
    /// The global is instanciated as a `Resource` and provided to the callback,
    /// do whatever you need with it.
    ///
    /// Letting it out of scope will *not* destroy the resource, and you'll still
    /// receive its events (as long as you've registered an appropriate handler).
    fn bind(&mut self, evqh: &mut EventLoopHandle, client: &Client, global: R);
}


pub struct EventLoopHandle {
    handlers: Vec<Box<Any>>,
    keep_going: bool,
}

impl EventLoopHandle {
    /// Register a resource to a handler of this event loop.
    ///
    /// The H type must be provided and match the type of the targetted Handler, or
    /// it will panic.
    ///
    /// This overwrites any precedently set Handler for this resource.
    pub fn register<R: Resource, H: Handler<R> + Any + 'static>(&mut self, resource: &R, handler_id: usize) {
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
                resource_destroy
            );
        }
    }

    /// Insert a new handler to this EventLoop
    ///
    /// Returns the index of this handler in the internal array, needed register
    /// proxies to it.
    pub fn add_handler<H: Any + 'static>(&mut self, handler: H) -> usize {
        self.handlers.push(Box::new(handler) as Box<Any>);
        self.handlers.len() - 1
    }
    
    pub fn stop_loop(&mut self) {
        self.keep_going = false;
    }
}

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
    pub fn get_mut_handler<H: Any + 'static>(&mut self, handler_id: usize) -> &H {
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
    /// If no request is pending, will block at most `timeout` ms is specified,
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
unsafe extern "C" fn resource_destroy(_resource: *mut wl_resource) {
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
            unsafe fn message(&mut self, evq: &mut $crate::EventLoopHandle, client: &$crate::CLient, proxy: &$handled_type, opcode: u32, args: *const $crate::sys::wl_argument) -> Result<(),()> {
                <$handler_struct as $handler_trait>::__message(self, evq, client, proxy, opcode, args)
            }
        }
    }
);
