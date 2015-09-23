extern crate crossbeam;

#[macro_use]
extern crate dlib;

#[cfg(feature = "dlopen")]
#[macro_use]
extern crate lazy_static;

extern crate libc;

mod abi;
mod events;
mod sys;

#[cfg(feature = "egl")]
pub mod egl;

pub mod wayland;

use abi::client::wl_proxy;
use abi::common::wl_interface;

pub use events::{Event, EventIterator};

pub trait Proxy {
    fn ptr(&self) -> *mut wl_proxy;
    fn interface() -> *mut wl_interface;
    /// The internal name of this interface, as advertized by the registry if it is a global.
    fn interface_name() -> &'static str;
    /// The maximum version of this interface handled by the library.
    fn version() -> u32;
    fn id(&self) -> ProxyId;
    unsafe fn from_ptr(ptr: *mut wl_proxy) -> Self;
    fn set_evt_iterator(&mut self, iter: &EventIterator);
}

#[derive(Copy,Clone,PartialEq,Eq,Debug)]
pub struct ProxyId { id: usize }

fn wrap_proxy(ptr: *mut wl_proxy) -> ProxyId {
    ProxyId { id: ptr as usize}
}