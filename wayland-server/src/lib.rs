extern crate wayland_sys;

pub struct EventQueueHandle;

pub use sys::server as protocol;

mod sys {
    #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
    #![allow(non_upper_case_globals,non_snake_case,unused_imports)]

    pub mod interfaces {
        include!(concat!(env!("OUT_DIR"), "/wayland_interfaces.rs"));
    }

    pub mod server {
        pub use EventQueueHandle;

        include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
    }
}
