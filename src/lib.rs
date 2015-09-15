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

pub trait Proxy {
    fn ptr(&self) -> *mut c_void;
}

#[derive(Copy,Clone,PartialEq,Eq)]
pub struct ProxyId { id: usize }