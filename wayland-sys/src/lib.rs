//! FFI bindings to the wayland system libraries.
//!
//! The names exported by this crate should *not* be used directly, but though
//! the `ffi_dispatch` macro like this:
//!
//! ```ignore
//! ffi_dispatch!(HANDLE_NAME, func_name, arg1, arg2, arg3);
//! ```
//!
//! Where `HANDLE_NAME` is the name of the handle generated if the cargo feature
//! `dlopen` is on.
//!
//! For this to work, you must ensure every needed symbol is in scope (aka the static handle
//! if `dlopen` is on, the extern function if not). The easiest way to do this is to glob import
//! the appropriate module. For example:
//!
//! ```no_run
//! #[macro_use] extern crate wayland_sys;
//!
//! use wayland_sys::client::*;
//!
//! fn main() {
//!     let display_ptr = unsafe {
//!         ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_connect, ::std::ptr::null())
//!     };
//! }
//! ```
//!
//! Each module except `common` corresponds to a system library. They all define a function named
//! `is_lib_available()` which returns a boolean depending on whether the lib could be loaded.
//! They always return true if the feature `dlopen` is absent, as the lib is then directly linked.

#![allow(dead_code, non_camel_case_types)]

#[macro_use]
extern crate dlib;

#[cfg(feature = "dlopen")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "server")]
extern crate libc;

/// Magic pointer for wayland objects managed by wayland-client or wayland-server
///
/// This static serves no purpose other than existing, and thus providing a stable pointer
/// to something we know what it is.
///
/// It is used internally by wayland-client, wayland-server and wayland-scanner to ensure safety
/// regarding to wayland objects that are created by some other library.
pub static RUST_MANAGED: u8 = 42;

pub mod common;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

#[cfg(all(feature = "egl", feature = "client"))]
pub mod egl;

#[cfg(all(feature = "cursor", feature = "client"))]
pub mod cursor;

#[cfg(feature = "server")]
pub use libc::{gid_t, pid_t, uid_t};

// Small hack while #[macro_reexport] is not stable

#[cfg(feature = "dlopen")]
#[macro_export]
macro_rules! ffi_dispatch(
    ($handle: ident, $func: ident, $($arg: expr),*) => (
        ($handle.$func)($($arg),*)
    )
);

#[cfg(not(feature = "dlopen"))]
#[macro_export]
macro_rules! ffi_dispatch(
    ($handle: ident, $func: ident, $($arg: expr),*) => (
        $func($($arg),*)
    )
);
