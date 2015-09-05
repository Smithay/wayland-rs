#![allow(dead_code,non_camel_case_types)]

pub mod interfaces {
    include!(concat!(env!("OUT_DIR"), "/wayland_interfaces.rs"));
}

pub mod client {
    include!(concat!(env!("OUT_DIR"), "/wayland_client_api.rs"));
}