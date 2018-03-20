#[macro_use]
extern crate bitflags;
extern crate libc;

extern crate wayland_commons;
#[cfg(feature = "native_lib")]
#[macro_use]
extern crate wayland_sys;

mod display;
mod event_queue;
mod proxy;

pub use proxy::{NewProxy, Proxy};
pub use display::Display;
pub use event_queue::{EventQueue, QueueToken};
#[cfg(feature = "native_lib")]
pub use generated::c_api as protocol;

#[cfg(feature = "native_lib")]
pub mod sys {
    pub use super::generated::c_interfaces as protocol_interfaces;
}

mod generated {
    #![allow(dead_code, non_camel_case_types, unused_unsafe, unused_variables)]
    #![allow(non_upper_case_globals, non_snake_case, unused_imports)]
    #![allow(missing_docs)]

    #[cfg(feature = "native_lib")]
    pub mod c_interfaces {
        include!(concat!(env!("OUT_DIR"), "/wayland_c_interfaces.rs"));
    }
    #[cfg(feature = "native_lib")]
    pub mod c_api {
        pub(crate) use {NewProxy, Proxy};
        pub(crate) use wayland_commons::{AnonymousObject, Interface, MessageGroup};
        pub(crate) use wayland_sys as sys;
        include!(concat!(env!("OUT_DIR"), "/wayland_c_api.rs"));
    }
}
