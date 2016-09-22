#[macro_use] extern crate bitflags;
#[macro_use] extern crate wayland_sys;
extern crate libc;

pub use generated::server as protocol;
pub use client::Client;
pub use display::{Display, create_display};
pub use event_loop::{EventLoop, EventLoopHandle, StateGuard, Global, GlobalHandler};

use wayland_sys::server::wl_resource;
use wayland_sys::common::{wl_interface, wl_argument};

mod client;
mod display;
mod event_loop;

pub trait Resource {
    /// Pointer to the underlying wayland proxy object
    fn ptr(&self) -> *mut wl_resource;
    /// Create an instance from a wayland pointer
    ///
    /// The pointer must refer to a valid wayland resource
    /// of the appropriate interface, but that have not yet
    /// been seen by the library.
    ///
    /// The library will take control of the object (notably
    /// overwrite its user_data).
    unsafe fn from_ptr_new(*mut wl_resource) -> Self;
    /// Create an instance from a wayland pointer
    ///
    /// The pointer must refer to a valid wayland resource
    /// of the appropriate interface, and have already been
    /// initialized by the library (it'll assume this proxy
    /// user_data contains a certain kind of data).
    unsafe fn from_ptr_initialized(*mut wl_resource) -> Self;
    /// Pointer to the interface representation
    fn interface_ptr() -> *const wl_interface;
    /// Internal wayland name of this interface
    fn interface_name() -> &'static str;
    /// Max version of this interface supported
    fn supported_version() -> u32;
    /// Current version of the interface this resource is instanciated with
    fn version(&self) -> i32;
    /// Check if the resource behind this handle is actually still alive
    fn is_alive(&self) -> bool;
}

/// Possible outcome of the call of a event on a resource
pub enum EventResult<T> {
    /// Message has been buffered and will be sent to client
    Sent(T),
    /// This resource is already destroyed, request has been ignored
    Destroyed
}

impl<T> EventResult<T> {
    pub fn expect(self, error: &str) -> T {
        match self {
            EventResult::Sent(v) => v,
            EventResult::Destroyed => panic!("{}", error)
        }
    }
}

pub unsafe trait Handler<T: Resource> {
    unsafe fn message(&mut self, evq: &mut EventLoopHandle, client: &Client, resource: &T, opcode: u32, args: *const wl_argument) -> Result<(),()>;
}

mod generated {
    #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
    #![allow(non_upper_case_globals,non_snake_case,unused_imports)]

    pub mod interfaces {
        include!(concat!(env!("OUT_DIR"), "/wayland_interfaces.rs"));
    }

    pub mod server {
        // Imports that need to be available to submodules
        // but should not be in public API.
        // Will be fixable with pub(restricted).
        #[doc(hidden)] pub use {Resource, EventLoopHandle, Handler, Client, EventResult};
        #[doc(hidden)] pub use super::interfaces;

        include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
    }
}

pub mod sys {
    pub use wayland_sys::server::*;
    pub use wayland_sys::common::*;
}
