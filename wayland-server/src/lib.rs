#![cfg_attr(feature = "unstable-protocols", feature(static_recursion))]

#[macro_use] extern crate bitflags;
extern crate crossbeam;
extern crate libc;
#[macro_use] extern crate wayland_sys;

mod client;
mod display;
#[macro_use]
mod globals;
mod requests;
mod sys;

pub mod wayland;

#[cfg(all(feature = "unstable-protocols", feature = "wpu-xdg_shell"))]
pub mod xdg_shell;

#[cfg(feature = "wl-desktop_shell")]
pub mod desktop_shell;

use wayland_sys::server::wl_resource;
use wayland_sys::common::wl_interface;

pub use display::Display;
pub use client::{Client, ClientId};

pub use requests::{Request, RequestIterator, IteratorDispatch, ResourceParent};
pub use globals::{Global, GlobalInstance, GlobalId, GlobalResource};

pub trait Resource {
    fn ptr(&self) -> *mut wl_resource;
    fn interface() -> *mut wl_interface;
    /// The internal name of this interface, as advertized by the registry if it is a global.
    fn interface_name() -> &'static str;
    /// The maximum version of the interface handled by this library
    fn max_version() -> u32;
    /// The version this resource has been bound with
    fn bound_version(&self) -> u32;
    /// Get the id of this resource
    fn id(&self) -> ResourceId;
    /// Get the id of the client associated with this resource
    fn client_id(&self) -> ClientId;
    /// Creates a proxy from a fresh ptr
    unsafe fn from_ptr(ptr: *mut wl_resource) -> Self;
    /// Creates a proxy from a ptr that is managed elsewhere
    ///
    /// As opposed to `from_ptr`, this function will not try to
    /// set a listener/dispatcher for this proxy, and thus its
    /// events won't be available.
    ///
    /// The created object _should_ be leaked, or it will destroy
    /// the ressource on drop, which will most likely trigger
    /// protocol errors.
    unsafe fn from_ptr_no_own(ptr: *mut wl_resource) -> Self;
    /// Set the request iterator associated to this proxy
    fn set_req_iterator(&mut self, iter: &RequestIterator);
}

#[derive(Copy,Clone,PartialEq,Eq,Debug,Hash)]
pub struct ResourceId { id: usize }

fn wrap_resource(ptr: *mut wl_resource) -> ResourceId {
    ResourceId { id: ptr as usize}
}
