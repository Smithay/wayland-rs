#![allow(non_camel_case_types, dead_code)]

pub mod abi;
pub mod enums;
pub mod interfaces;

#[doc(hidden)]
pub trait FFI<T> {
    fn ptr(&self) -> *const T;
    unsafe fn ptr_mut(&self) -> *mut T;
}