//! Rust implementations of the Wayland backends

mod client_impl;
mod server_impl;

mod debug;
mod map;
pub(crate) mod socket;
mod wire;

/// Client-side rust implementation of a Wayland protocol backend
#[path = "../client_api.rs"]
pub mod client;

/// Server-side rust implementation of a Wayland protocol backend
#[path = "../server_api.rs"]
pub mod server;
