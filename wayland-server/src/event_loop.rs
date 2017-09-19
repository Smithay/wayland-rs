use {Client, Implementable, Resource};
use std::any::Any;
use std::cell::Cell;
use std::io::{Error as IoError, Result as IoResult};
use std::io::Write;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_int, c_void};
use std::os::unix::io::RawFd;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr};
use wayland_sys::RUST_MANAGED;
use wayland_sys::common::{wl_argument, wl_message};
use wayland_sys::server::*;

type ResourceUserData = (
    *mut EventLoopHandle,
    Option<Box<Any>>,
    Arc<(AtomicBool, AtomicPtr<()>)>,
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

/// A state store
///
/// This struct allows you to store various values in a special
/// storage that will be made available to your proxy implementations.
pub struct State {
    values: Vec<Option<(Box<Any>, Rc<Cell<bool>>)>>,
}

/// A token for accessing the state store contents
pub struct StateToken<V> {
    id: usize,
    live: Rc<Cell<bool>>,
    _type: PhantomData<V>,
}

impl<V> Clone for StateToken<V> {
    fn clone(&self) -> StateToken<V> {
        StateToken {
            id: self.id,
            live: self.live.clone(),
            _type: PhantomData,
        }
    }
}

impl State {
    /// Insert a new value in this state store
    ///
    /// Returns a clonable token that you can later use to access this
    /// value.
    pub fn insert<V: Any + 'static>(&mut self, value: V) -> StateToken<V> {
        let boxed = Box::new(value) as Box<Any>;
        let live = Rc::new(Cell::new(true));
        {
            // artificial scope to make the borrow checker happy
            let empty_slot = self.values
                .iter_mut()
                .enumerate()
                .find(|&(_, ref s)| s.is_none());
            if let Some((id, slot)) = empty_slot {
                *slot = Some((boxed, live.clone()));
                return StateToken {
                    id: id,
                    live: live,
                    _type: PhantomData,
                };
            }
        }
        self.values.push(Some((boxed, live.clone())));
        StateToken {
            id: self.values.len() - 1,
            live: live,
            _type: PhantomData,
        }
    }

    /// Access value previously inserted in this state store
    ///
    /// Panics if the provided token corresponds to a value that was removed.
    pub fn get<V: Any + 'static>(&self, token: &StateToken<V>) -> &V {
        if !token.live.get() {
            panic!("Attempted to access a state value that was already removed!");
        }
        self.values[token.id]
            .as_ref()
            .and_then(|t| t.0.downcast_ref::<V>())
            .unwrap()
    }

    /// Mutably access value previously inserted in this state store
    ///
    /// Panics if the provided token corresponds to a value that was removed.
    pub fn get_mut<V: Any + 'static>(&mut self, token: &StateToken<V>) -> &mut V {
        if !token.live.get() {
            panic!("Attempted to access a state value that was already removed!");
        }
        self.values[token.id]
            .as_mut()
            .and_then(|t| t.0.downcast_mut::<V>())
            .unwrap()
    }

    /// Remove a value previously inserted in this state store
    ///
    /// Panics if the provided token corresponds to a value that was already
    /// removed.
    pub fn remove<V: Any + 'static>(&mut self, token: StateToken<V>) -> V {
        if !token.live.get() {
            panic!("Attempted to remove a state value that was already removed!");
        }
        let (boxed, live) = self.values[token.id].take().unwrap();
        live.set(false);
        *boxed.downcast().unwrap()
    }
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
    state: State,
    keep_going: bool,
    ptr: *mut wl_event_loop,
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
    pub fn register<R, ID>(&mut self, resource: &R, implementation: R::Implementation, idata: ID)
                           -> RegisterStatus
    where
        R: Resource + Implementable<ID>,
    {
        self.register_with_destructor::<R, ID, NoopDestroy>(resource, implementation, idata)
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
    pub fn register_with_destructor<R, ID, D>(&mut self, resource: &R, implementation: R::Implementation,
                                              idata: ID)
                                              -> RegisterStatus
    where
        R: Resource + Implementable<ID>,
        D: Destroy<R> + 'static,
    {
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
            (&mut *data).1 = Some(Box::new((implementation, idata)) as Box<Any>);
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_set_dispatcher,
                resource.ptr(),
                dispatch_func::<R, ID>,
                &RUST_MANAGED as *const _ as *const _,
                data as *mut c_void,
                Some(resource_destroy::<R, D>)
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
        ) as *mut ResourceUserData)
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
        display: display,
        handle: Box::new(EventLoopHandle {
            state: State { values: Vec::new() },
            keep_going: false,
            ptr: ptr,
        }),
    }
}

pub struct EventLoop {
    display: Option<*mut wl_display>,
    handle: Box<EventLoopHandle>,
}

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
        let display = self.display.expect(
            "Globals can only be registered on an event loop associated with a display.",
        );

        let data = Box::new((
            callback,
            &*self.handle as *const _ as *mut EventLoopHandle,
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
        (data.2)
            .0
            .store(false, ::std::sync::atomic::Ordering::SeqCst);
    }
    D::destroy(&resource);
}
