#[macro_use]
extern crate bitflags;
extern crate libc;

extern crate wayland_commons;
#[cfg(feature = "native_lib")]
#[macro_use]
extern crate wayland_sys;

mod resource;
pub use resource::{NewResource, Resource};

struct Client;

mod generated {
    #![allow(dead_code, non_camel_case_types, unused_unsafe, unused_variables)]
    #![allow(non_upper_case_globals, non_snake_case, unused_imports)]
    #![allow(missing_docs)]

    #[cfg(feature = "native_lib")]
    mod c_interfaces {
        include!(concat!(env!("OUT_DIR"), "/wayland_c_interfaces.rs"));
    }
    #[cfg(feature = "native_lib")]
    mod c_api {
        include!(concat!(env!("OUT_DIR"), "/wayland_c_api.rs"));
    }
}
