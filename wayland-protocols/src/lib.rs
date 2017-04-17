//! This crate provides bindings to the official wayland protocol extensions
//! provided in https://cgit.freedesktop.org/wayland/wayland-protocols
//!
//! These bindings are built on top of the crates wayland-client and wayland-server.
//!
//! Each protocol module contains a `client` and a `server` submodules, for each side of the
//! protocol. The creation of these modules (and the dependency on the associated crate) is
//! controlled by the two cargo features `client` and `server`.
//!
//! The cargo feature `unstable_protocols` adds an `unstable` module, containings bindings
//! to protocols that are not yet considered stable. As such, no stability guarantee is
//! given for these protocols.
//!
//! Some protocols require unstable rust features, the inclusion of them is controlled
//! by the cargo feature `nightly`.

#![warn(missing_docs)]

#[cfg(feature = "client")]
extern crate wayland_client;

#[cfg(feature = "server")]
extern crate wayland_server;

#[macro_use]
extern crate wayland_sys;

#[macro_use]
extern crate bitflags;

#[macro_use]
mod protocol_macro;

#[cfg(feature = "unstable_protocols")]
pub mod unstable;

mod stable;
pub use stable::*;
