#[cfg(feature = "native_lib")]
use wayland_sys::server::wl_client;

use imp::ClientInner;

/// A handle to a client connected to your server
///
/// There can be several handles referring to the same client
#[derive(Clone)]
pub struct Client {
    inner: ClientInner,
}

impl Client {
    #[cfg(feature = "native_lib")]
    /// Create a client from a `wayland-server.so` pointer
    pub unsafe fn from_ptr(ptr: *mut wl_client) -> Client {
        Client {
            inner: ClientInner::from_ptr(ptr),
        }
    }

    #[cfg(feature = "native_lib")]
    /// Retrieve a pointer to the underlying `wl_client` of `wayland-server.so`
    pub fn c_ptr(&self) -> *mut wl_client {
        self.inner.ptr()
    }

    pub(crate) fn make(inner: ClientInner) -> Client {
        Client { inner }
    }

    /// Check whether this client is still connected to the server
    pub fn alive(&self) -> bool {
        self.inner.alive()
    }

    /// Check whether this client handle refers to the same client as
    /// an other
    pub fn equals(&self, other: &Client) -> bool {
        self.inner.equals(&other.inner)
    }

    /// Flush the pending events to this client
    pub fn flush(&self) {
        self.inner.flush()
    }

    /// Kills this client
    ///
    /// Does nothing if the client is already dead
    pub fn kill(&self) {
        self.inner.kill()
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
        self.inner.set_user_data(data)
    }

    /// Retrieve the arbitrary payload associated to this client
    ///
    /// See `set_user_data` for explanations.
    pub fn get_user_data(&self) -> *mut () {
        self.inner.get_user_data()
    }

    /// Set a destructor for this client
    ///
    /// the provided function will be called when the client disconnects
    /// or is killed. It's argument is what you would get from calling
    /// `get_user_data`.
    pub fn set_destructor(&self, destructor: fn(*mut ())) {
        self.inner.set_destructor(destructor)
    }
}
