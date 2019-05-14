#[cfg(feature = "native_lib")]
use wayland_sys::server::wl_client;

use imp::ClientInner;

use {Interface, NewResource, UserDataMap};

/// A handle to a client connected to your server
///
/// There can be several handles referring to the same client.
#[derive(Clone)]
pub struct Client {
    inner: ClientInner,
}

impl Client {
    #[cfg(feature = "native_lib")]
    /// Creates a client from a `wayland-server.so` pointer
    pub unsafe fn from_ptr(ptr: *mut wl_client) -> Client {
        Client {
            inner: ClientInner::from_ptr(ptr),
        }
    }

    #[cfg(feature = "native_lib")]
    /// Returns a pointer to the underlying `wl_client` of `wayland-server.so`
    pub fn c_ptr(&self) -> *mut wl_client {
        self.inner.ptr()
    }

    pub(crate) fn make(inner: ClientInner) -> Client {
        Client { inner }
    }

    /// Checks whether this client is still connected to the server
    pub fn alive(&self) -> bool {
        self.inner.alive()
    }

    /// Checks whether `self` and `other` refer to the same client
    pub fn equals(&self, other: &Client) -> bool {
        self.inner.equals(&other.inner)
    }

    /// Flushes the pending events to this client
    pub fn flush(&self) {
        self.inner.flush()
    }

    /// Kills this client
    ///
    /// Does nothing if the client is already dead.
    pub fn kill(&self) {
        self.inner.kill()
    }

    /// Returns a reference to the `UserDataMap` associated with this client
    ///
    /// See `UserDataMap` documentation for details about its use.
    pub fn data_map(&self) -> &UserDataMap {
        self.inner.user_data_map()
    }

    /// Adds a destructor for this client
    ///
    /// This closure will be called when the client disconnects or is killed.
    /// It has access to the `UserDataMap` associated with this client.
    ///
    /// You can add several destructors which will all be called sequentially. Note
    /// that if you accidentally add two copies of the same closure, it will be called
    /// twice.
    ///
    /// The destructors will be executed on the thread containing the wayland event loop.
    pub fn add_destructor<F: FnOnce(&UserDataMap) + Send + 'static>(&self, destructor: F) {
        self.inner.add_destructor(destructor)
    }

    /// Creates a new resource for this client
    ///
    /// To ensure the state coherence between client and server, this
    /// resource should immediately be implemented and sent to the client
    /// through an appropriate event. Failure to do so will likely cause
    /// protocol errors.
    pub fn create_resource<I: Interface>(&self, version: u32) -> Option<NewResource<I>> {
        self.inner.create_resource::<I>(version).map(NewResource::wrap)
    }
}
