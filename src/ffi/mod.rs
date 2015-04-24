#![allow(non_camel_case_types, dead_code)]

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