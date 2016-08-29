extern crate wayland_sys;

pub use sys::client as protocol;

use wayland_sys::client::wl_proxy;

pub trait Proxy {
    fn ptr(&self) -> *mut wl_proxy;
    unsafe fn from_ptr(*mut wl_proxy) -> Self;
}

pub struct EventQueueHandle;

mod sys {
    #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
    #![allow(non_upper_case_globals,non_snake_case,unused_imports)]

    pub mod interfaces {
        include!(concat!(env!("OUT_DIR"), "/wayland_interfaces.rs"));
    }

    pub mod client {
        pub use Proxy;
        pub use EventQueueHandle;
        include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
    }
}
