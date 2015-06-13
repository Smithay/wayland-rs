//! This library provides a Rust interface on the Wayland client library.
//!
//! To use it, you'll need to have `libwayland-client.so` available on your system.
//! However, to allow for easy optionnal support of wayland in applications, the library
//! is not linked, but will be opened at first use of a wayland method.
//!
//! You can check the presence of the library with the function `is_wayland_lib_available()`,
//! and the methods creating the `Display` object will not panic if the library is absent, but
//! return a failure (`None` or `Err(_)`).
//!
//! The module `core` provides support for the core features of the wayland protocol.
//! Some protocol extentions are available, each as their own module. Some of them require
//! a system library which they will try to load at first use.
//!
//! - module `egl`: it provides a mean to build EGL surfaces in a wayland context. It requires
//!   the presence of `libwayland-egl.so`. This module is activated by the `egl` feature.
//!
//! Additionnaly, the feature `dlopen` prevents the crate to be linked to the various
//! wayland libraries. In this case, it will instead try to load them dynamically at runtime.
//! In this case, each module will provide a function allowing you to gracefully check if
//! the load was successful. There is also the function `is_wayland_lib_available()` at the
//! root of this crate, providing the same function for the core `libwayland-client.so`.

#[macro_use] extern crate bitflags;
#[macro_use] extern crate lazy_static;
extern crate libc;

#[macro_use] mod ffi;

pub mod core;

#[cfg(feature = "egl")]
pub mod egl;

pub mod internals {
    //! Internal types and traits provided for special cases like custom protocol extensions.
    pub use ffi::abi::{wl_interface, wl_message};
    pub use ffi::{FFI, Bind};
}

#[cfg(feature = "dlopen")]
/// Returns whether the library `libwayland-client.so` has been found and could be loaded.
///
/// This function is present only if the feature `dlopen` is activated
pub fn is_wayland_lib_available() -> bool {
    ffi::abi::WAYLAND_CLIENT_OPTION.is_some()
}