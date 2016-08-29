extern crate wayland_sys;

pub use sys::server as protocol;

use wayland_sys::server::wl_resource;

pub trait Resource {
    fn ptr(&self) -> *mut wl_resource;
    unsafe fn from_ptr(*mut wl_resource) -> Self;
}

pub struct EventQueueHandle;

mod sys {
    #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
    #![allow(non_upper_case_globals,non_snake_case,unused_imports)]

    pub mod interfaces {
        include!(concat!(env!("OUT_DIR"), "/wayland_interfaces.rs"));
    }

    pub mod server {
        pub use Resource;
        pub use EventQueueHandle;

        include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
    }
}
