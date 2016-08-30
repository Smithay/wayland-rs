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
        // Imports that need to be available to submodules
        // but should not be in public API.
        // Will be fixable with pub(restricted).
        #[doc(hidden)] pub use Resource;
        #[doc(hidden)] pub use EventQueueHandle;

        include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
    }
}
