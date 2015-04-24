//! This library provides a Rust interface on the Wayland client library.
//!
//! To use it, you'll need to have `libwayland-client.so` available on your system.
//!
//! The module `core` provides support for the core features of the wayland protocol.
//! Some protocol extentions are available and can each be activated using the appropriate
//! cargo feature:
//!
//! - module `egl`: it provides a mean to build EGL surfaces in a wayland context. It requires
//!   the presence of `libwayland-egl.so`, provided by mesa. It can be activated with the cargo
//!   feature `wl_egl`.
//!
//! The special feature `all` can also be used to activate all

extern crate libc;

mod ffi;

pub mod core;
#[cfg(feature = "wl_egl")]
pub mod egl;