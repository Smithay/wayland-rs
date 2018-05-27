use std::os::raw::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;

#[cfg(feature = "native_lib")]
use wayland_sys::server::*;

pub(crate) struct ClientInternal {
    alive: AtomicBool,
    user_data: AtomicPtr<()>,
    destructor: AtomicPtr<()>,
}

impl ClientInternal {
    fn new() -> ClientInternal {
        ClientInternal {
            alive: AtomicBool::new(true),
            user_data: AtomicPtr::new(::std::ptr::null_mut()),
            destructor: AtomicPtr::new(::std::ptr::null_mut()),
        }
    }
}

/// A handle to a client connected to your server
///
/// There can be several handles referring to the same client
#[derive(Clone)]
pub struct Client {
    internal: Arc<ClientInternal>,
    #[cfg(feature = "native_lib")]
    ptr: *mut wl_client,
}

impl Client {
    #[cfg(feature = "native_lib")]
    /// Create a client from a `wayland-server.so` pointer
    pub unsafe fn from_ptr(ptr: *mut wl_client) -> Client {
        // check if we are already registered
        let listener = ffi_dispatch!(
            WAYLAND_SERVER_HANDLE,
            wl_client_get_destroy_listener,
            ptr,
            client_destroy
        );
        if listener.is_null() {
            // need to init this client
            let listener = signal::rust_listener_create(client_destroy);
            let internal = Arc::new(ClientInternal::new());
            signal::rust_listener_set_user_data(
                listener,
                Box::into_raw(Box::new(internal.clone())) as *mut c_void,
            );
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_client_add_destroy_listener,
                ptr,
                listener
            );
            Client {
                internal: internal,
                ptr: ptr,
            }
        } else {
            // client already initialized
            let internal = signal::rust_listener_get_user_data(listener) as *mut Arc<ClientInternal>;
            Client {
                internal: (*internal).clone(),
                ptr: ptr,
            }
        }
    }

    #[cfg(feature = "native_lib")]
    /// Retrieve a pointer to the underlying `wl_client` of `wayland-server.so`
    pub fn c_ptr(&self) -> *mut wl_client {
        self.ptr
    }

    /// Check whether this client is still connected to the server
    pub fn alive(&self) -> bool {
        self.internal.alive.load(Ordering::Acquire)
    }

    /// Check whether this client handle refers to the same client as
    /// an other
    pub fn equals(&self, other: &Client) -> bool {
        Arc::ptr_eq(&self.internal, &other.internal)
    }

    /// Flush the pending events to this client
    pub fn flush(&self) {
        if !self.alive() {
            return;
        }
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_flush, self.ptr);
        }
    }

    /// Kills this client
    ///
    /// Does nothing if the client is already dead
    pub fn kill(&self) {
        if !self.alive() {
            return;
        }
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_destroy, self.ptr);
        }
    }

    /// Associate an arbitrary payload to this client
    ///
    /// The pointer you associate here can be retrieved from any
    /// other handle to the same client.
    ///
    /// Setting or getting user data is done as an atomic operation.
    /// You are responsible for the correct initialization of this
    /// pointer, synchronisation of access, and destruction of the
    /// contents at the appropriate time.
    pub fn set_user_data(&self, data: *mut ()) {
        self.internal.user_data.store(data, Ordering::Release);
    }

    /// Retrieve the arbitrary payload associated to this client
    ///
    /// See `set_user_data` for explanations.
    pub fn get_user_data(&self) -> *mut () {
        self.internal.user_data.load(Ordering::Acquire)
    }

    /// Set a destructor for this client
    ///
    /// the provided function will be called when the client disconnects
    /// or is killed. It's argument is what you would get from calling
    /// `get_user_data`.
    pub fn set_destructor(&self, destructor: fn(*mut ())) {
        self.internal
            .destructor
            .store(destructor as *const () as *mut (), Ordering::Release);
    }
}

#[cfg(feature = "native_lib")]
unsafe extern "C" fn client_destroy(listener: *mut wl_listener, _data: *mut c_void) {
    let internal = Box::from_raw(signal::rust_listener_get_user_data(listener) as *mut Arc<ClientInternal>);
    signal::rust_listener_set_user_data(listener, ptr::null_mut());
    // Store that we are dead
    internal.alive.store(false, Ordering::Release);
    // TODO: call the user callback
    let destructor = internal.destructor.load(Ordering::Acquire);
    if !destructor.is_null() {
        let destructor_func: fn(*mut ()) = ::std::mem::transmute(destructor);
        destructor_func(internal.user_data.load(Ordering::Acquire));
    }
    signal::rust_listener_destroy(listener);
}
