#![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
#![allow(non_upper_case_globals,non_snake_case,unused_imports)]

pub mod interfaces {
    use sys::wayland::interfaces::{wl_surface_interface, wl_output_interface};
    include!(concat!(env!("OUT_DIR"), "/desktop_shell_interfaces.rs"));
}

pub mod server {
    use sys::wayland::server::{WlOutput, WlSurface};
    include!(concat!(env!("OUT_DIR"), "/desktop_shell_server_api.rs"));
}
