#![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
#![allow(non_upper_case_globals,non_snake_case,unused_imports)]

pub mod interfaces {
    use sys::wayland::interfaces::wl_surface_interface;
    include!(concat!(env!("OUT_DIR"), "/viewporter_interfaces.rs"));
}

pub mod client {
    use sys::wayland::client::WlSurface;
    include!(concat!(env!("OUT_DIR"), "/viewporter_client_api.rs"));
}