#![feature(static_recursion, result_expect)]

#[macro_use]
extern crate dlib;

#[cfg(feature = "dlopen")]
#[macro_use]
extern crate lazy_static;

extern crate libc;

mod abi;
mod wayland;

use abi::client::wl_proxy;
use abi::common::wl_interface;

pub trait Proxy {
    type Id : Into<ProxyId>;
    fn ptr(&self) -> *mut wl_proxy;
    fn interface() -> *mut wl_interface;
    fn id(&self) -> Self::Id;
    unsafe fn from_ptr(ptr: *mut wl_proxy) -> Self;
}

#[derive(Copy,Clone,PartialEq,Eq)]
pub struct ProxyId { id: usize }
