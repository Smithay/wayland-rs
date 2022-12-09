//! Implementations of the Wayland backends using the system `libwayland`

use crate::protocol::ArgumentType;
use wayland_sys::common::{wl_argument, wl_array};

#[cfg(any(test, feature = "client_system"))]
mod client_impl;
#[cfg(any(test, feature = "server_system"))]
mod server_impl;

/// Magic static for wayland objects managed by wayland-client or wayland-server
///
/// This static serves no purpose other than existing at a stable address.
static RUST_MANAGED: u8 = 42;

unsafe fn free_arrays(signature: &[ArgumentType], arglist: &[wl_argument]) {
    for (typ, arg) in signature.iter().zip(arglist.iter()) {
        if let ArgumentType::Array = typ {
            // Safety: the arglist provided arglist must be valid for associated signature
            // and contains pointers to boxed arrays as appropriate
            let _ = unsafe { Box::from_raw(arg.a as *mut wl_array) };
        }
    }
}

/// Client-side implementation of a Wayland protocol backend using `libwayland`
///
/// Entrypoints are:
/// - [`Backend::connect`](client::Backend::connect) method if you're creating the Wayland connection
/// - [`Backend::from_foreign_display`](client::Backend::from_foreign_display) if you're interacting with an
///   already existing Wayland connection through FFI.
#[cfg(any(test, feature = "client_system"))]
#[path = "../client_api.rs"]
pub mod client;

// API complements for FFI

#[cfg(any(test, feature = "client_system"))]
impl client::ObjectId {
    /// Creates an object id from a libwayland-client pointer.
    ///
    /// # Errors
    ///
    /// This function returns an [`InvalidId`](client::InvalidId) error if the interface of the proxy does
    /// not match the provided interface.
    ///
    /// # Safety
    ///
    /// The provided pointer must be a valid pointer to a `wl_resource` and remain valid for as
    /// long as the retrieved `ObjectId` is used.
    pub unsafe fn from_ptr(
        interface: &'static crate::protocol::Interface,
        ptr: *mut wayland_sys::client::wl_proxy,
    ) -> Result<Self, client::InvalidId> {
        Ok(Self { id: unsafe { client_impl::InnerObjectId::from_ptr(interface, ptr) }? })
    }

    /// Get the underlying libwayland pointer for this object
    pub fn as_ptr(&self) -> *mut wayland_sys::client::wl_proxy {
        self.id.as_ptr()
    }
}

#[cfg(any(test, feature = "client_system"))]
impl client::Backend {
    /// Creates a Backend from a foreign `*mut wl_display`.
    ///
    /// This is useful if you are writing a library that is expected to plug itself into an existing
    /// Wayland connection.
    ///
    /// This will initialize the [`Backend`](client::Backend) in "guest" mode, meaning it will not close the
    /// connection on drop. After the [`Backend`](client::Backend) is dropped, if the server sends an event
    /// to an object that was created from it, that event will be silently discarded. This may lead to
    /// protocol errors if the server expects an answer to that event, as such you should make sure to
    /// cleanup your Wayland state before dropping the [`Backend`](client::Backend).
    ///
    /// # Safety
    ///
    /// You need to ensure the `*mut wl_display` remains live as long as the  [`Backend`](client::Backend)
    /// (or its clones) exist.
    pub unsafe fn from_foreign_display(display: *mut wayland_sys::client::wl_display) -> Self {
        Self { backend: unsafe { client_impl::InnerBackend::from_foreign_display(display) } }
    }

    /// Returns the underlying `wl_display` pointer to this backend.
    ///
    /// This pointer is needed to interface with EGL, Vulkan and other C libraries.
    ///
    /// This pointer is only valid for the lifetime of the backend.
    pub fn display_ptr(&self) -> *mut wayland_sys::client::wl_display {
        self.backend.display_ptr()
    }
}

// SAFETY:
// - The display_ptr will not change for the lifetime of the backend.
// - The display_ptr will be valid, either because we have created the pointer or the caller which created the
//   backend has ensured the pointer is valid when `Backend::from_foreign_display` was called.
#[cfg(feature = "raw-window-handle")]
unsafe impl raw_window_handle::HasRawDisplayHandle for client::Backend {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        let mut handle = raw_window_handle::WaylandDisplayHandle::empty();
        handle.display = self.display_ptr().cast();
        raw_window_handle::RawDisplayHandle::Wayland(handle)
    }
}

/// Server-side implementation of a Wayland protocol backend using `libwayland`
///
/// The main entrypoint is the [`Backend::new`](server::Backend::new) method.
#[cfg(any(test, feature = "server_system"))]
#[path = "../server_api.rs"]
pub mod server;

#[cfg(any(test, feature = "server_system"))]
impl server::ObjectId {
    /// Creates an object from a C pointer.
    ///
    /// # Errors
    ///
    /// This function returns an [`InvalidId`](server::InvalidId) error if the interface of the
    /// resource does not match the provided interface.
    ///
    /// # Safety
    ///
    /// The provided pointer must be a valid pointer to a `wl_resource` and remain valid for as
    /// long as the retrieved `ObjectId` is used.
    pub unsafe fn from_ptr(
        interface: &'static crate::protocol::Interface,
        ptr: *mut wayland_sys::server::wl_resource,
    ) -> Result<Self, server::InvalidId> {
        Ok(Self { id: unsafe { server_impl::InnerObjectId::from_ptr(Some(interface), ptr) }? })
    }

    /// Returns the pointer that represents this object.
    ///
    /// The pointer may be used to interoperate with libwayland.
    pub fn as_ptr(&self) -> *mut wayland_sys::server::wl_resource {
        self.id.as_ptr()
    }
}

#[cfg(any(test, feature = "server_system"))]
impl server::Handle {
    /// Access the underlying `*mut wl_display` pointer
    pub fn display_ptr(&self) -> *mut wayland_sys::server::wl_display {
        self.handle.display_ptr()
    }
}
