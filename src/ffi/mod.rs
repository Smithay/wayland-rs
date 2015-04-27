#![allow(non_camel_case_types, dead_code)]

use libc::{c_int, c_char, c_void};

#[macro_use] mod dlopen;

pub mod abi;
pub mod enums;
pub mod interfaces;

#[doc(hidden)]
/// A trait for structures wrapping a FFI pointer, to access the pointer.
pub trait FFI {
    type Ptr;
    fn ptr(&self) -> *const <Self as FFI>::Ptr;
    unsafe fn ptr_mut(&self) -> *mut <Self as FFI>::Ptr;
}

#[doc(hidden)]
/// A trait for structure representing global objects that can be bound
/// by the registry.
pub trait Bind<'a, R> : FFI {
    #[inline]
    fn interface() -> &'static abi::wl_interface;
    #[inline]
    unsafe fn wrap(ptr: *mut <Self as FFI>::Ptr, parent: &'a R) -> Self;
}

extern {
    pub fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    pub fn dlerror() -> *mut c_char;
    pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    pub fn dlclose(handle: *mut c_void) -> c_int;
}