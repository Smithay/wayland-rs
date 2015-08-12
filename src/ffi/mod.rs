#![allow(non_camel_case_types, dead_code)]

pub mod abi;
pub mod enums;
pub mod interfaces;

/// A trait for structures wrapping a FFI pointer, to access the pointer.
///
/// Normal use of the library does not require using this trait, it is only
/// provided for special situations like EGL requiring the pointer to the
/// `wl_display`, or implementing custom protocol extentions.
pub trait FFI {
    type Ptr;
    /// Returns a `*const` pointer to the underlying wayland object.
    fn ptr(&self) -> *const <Self as FFI>::Ptr;
    /// Returns a `*mut` pointer to the underlying wayland object.
    unsafe fn ptr_mut(&self) -> *mut <Self as FFI>::Ptr;
}

/// A trait for structure representing global objects that can be bound
/// by the registry.
///
/// Normal use of the library does not require using this trait, it is only
/// provided for special situations like implementing custom protocol extentions.
pub trait Bind<R> : FFI {
    /// The `wl_interface` used to create this object in the registry.
    #[inline]
    fn interface() -> &'static abi::wl_interface;
    /// Create the object by wraping the pointer returned by the registry.
    ///
    /// `parent` is a reference to the registry, its primary role is to allow
    /// the newly created object to keep a clone of the regitry, in order
    /// to keep the wayland connexion alive.
    #[inline]
    unsafe fn wrap(ptr: *mut <Self as FFI>::Ptr, parent: R) -> Self;
}