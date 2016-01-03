#[macro_use] extern crate bitflags;
extern crate crossbeam;
#[macro_use] extern crate dlib;
extern crate libc;
extern crate wayland_sys;

mod env;
mod events;
mod sys;

#[cfg(feature = "egl")]
pub mod egl;

#[cfg(feature = "cursor")]
pub mod cursor;

pub mod wayland;

use wayland_sys::client::wl_proxy;
use wayland_sys::common::wl_interface;

pub use events::{Event, EventIterator};

pub trait Proxy {
    fn ptr(&self) -> *mut wl_proxy;
    fn interface() -> *mut wl_interface;
    /// The internal name of this interface, as advertized by the registry if it is a global.
    fn interface_name() -> &'static str;
    /// The maximum version of this interface handled by the library.
    fn version() -> u32;
    /// Get the id of this proxy
    fn id(&self) -> ProxyId;
    /// Creates a proxy from a fresh ptr
    unsafe fn from_ptr(ptr: *mut wl_proxy) -> Self;
    /// Creates a proxy from a ptr that is managed elsewhere
    ///
    /// As opposed to `from_ptr`, this function will not try to
    /// set a listener/dispatcher for this proxy, and thus its
    /// events won't be available.
    unsafe fn from_ptr_no_own(ptr: *mut wl_proxy) -> Self;
    /// Set the event iterator associated to this proxy
    fn set_evt_iterator(&mut self, iter: &EventIterator);
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
