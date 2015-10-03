#![allow(dead_code, non_camel_case_types)]

#[macro_use]
extern crate dlib;

#[cfg(feature = "dlopen")]
#[macro_use]
extern crate lazy_static;

extern crate libc;

pub mod common;
pub mod client;

#[cfg(feature = "egl")]
pub mod egl;