#![feature(static_recursion)]

#[macro_use]
extern crate dlib;

#[cfg(feature = "dlopen")]
#[macro_use]
extern crate lazy_static;

extern crate libc;

mod abi;
mod wayland;

use libc::c_void;
use abi::common::wl_interface;

pub trait Proxy {
    type Id : Into<ProxyId>;
    fn ptr(&self) -> *mut c_void;
    fn interface() -> *mut wl_interface;
    fn id(&self) -> Self::Id;
}

#[derive(Copy,Clone,PartialEq,Eq)]
pub struct ProxyId { id: usize }
