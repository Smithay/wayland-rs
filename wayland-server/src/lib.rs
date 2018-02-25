extern crate libc;

extern crate wayland_commons;
#[cfg(feature = "native_lib")]
#[macro_use]
extern crate wayland_sys;

mod resource;
pub use resource::{Resource, NewResource};

struct Client;
