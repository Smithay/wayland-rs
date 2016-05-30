#![cfg_attr(feature = "unstable-protocols", feature(static_recursion))]

#![deny(missing_docs)]

#[macro_use] extern crate bitflags;
extern crate libc;
#[macro_use] extern crate wayland_sys;

use std::sync::Arc;

mod env;
mod events;
mod sys;

#[cfg(feature = "egl")]
pub mod egl;

#[cfg(feature = "cursor")]
pub mod cursor;

pub mod wayland;

pub mod extensions;

use wayland_sys::client::wl_proxy;
use wayland_sys::common::wl_interface;

pub use wayland::{get_display, ConnectError};

pub use events::{Event, EventIterator, ReadEventsGuard};
use events::EventFifo;

pub trait Proxy : ProxyInternal {
    fn ptr(&self) -> *mut wl_proxy;
    fn interface() -> *mut wl_interface;
    /// The internal name of this interface, as advertized by the registry if it is a global.
    fn interface_name() -> &'static str;
    /// The maximum version of this interface handled by the library.
    fn version() -> u32;
    /// Get the id of this proxy
    fn id(&self) -> ProxyId;
    /// Set the event iterator associated to this proxy
    fn set_event_iterator(&mut self, iter: &EventIterator);
}

/// Trait used internally for implementation details.
#[doc(hidden)]
pub trait ProxyInternal {
    /// Creates a proxy from a fresh ptr
    unsafe fn from_ptr(ptr: *mut wl_proxy) -> Self;
    /// Creates a proxy from a ptr that is managed elsewhere
    ///
    /// As opposed to `from_ptr`, this function will not try to
    /// set a listener/dispatcher for this proxy, and thus its
    /// events won't be available.
    ///
    /// The created object _should_ be leaked, or it will destroy
    /// the ressource on drop, which will most likely trigger
    /// protocol errors.
    unsafe fn from_ptr_no_own(ptr: *mut wl_proxy) -> Self;
    /// Sets the event queue manually
    unsafe fn set_evq(&mut self, internals: Arc<EventFifo>);
}

#[derive(Copy,Clone,PartialEq,Eq,Debug,Hash)]
pub struct ProxyId { id: usize }

fn wrap_proxy(ptr: *mut wl_proxy) -> ProxyId {
    ProxyId { id: ptr as usize}
}

/// Checks if the wayland lib is available
///
/// If the `dlopen` feature is disabled, will always return `true`.
/// If it is enabled, will return `true` if the wayland-client lib
/// is available and can be used.
pub fn is_available() -> bool {
    ::wayland_sys::client::is_lib_available()
}
