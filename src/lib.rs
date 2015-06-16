//! This library provides a Rust interface on the Wayland client library.
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
//!
//! The entry point of your wayland application will be `core::default_display()`, which will
//! provide you with a `Display` object representing the connexion to the wayland server.
//! This display will give you access to the `Registry`, which will then give you access to the
//! various Wayland global objects.

#[macro_use] extern crate bitflags;
#[macro_use] extern crate lazy_static;
extern crate libc;

#[macro_use] mod ffi;

pub mod core;

#[cfg(feature = "egl")]
pub mod egl;

pub mod internals {
    //! Internal types and traits provided for custom protocol extensions.
    //!
    //! You most likely won't need to use these, unless you plan to plug a custom
    //! wayland protocol into this library, in which case this interface should be
    //! enough to plug the Registry into working with it.
    //!
    //! If not, don't hesitate to open an issue on Github.
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