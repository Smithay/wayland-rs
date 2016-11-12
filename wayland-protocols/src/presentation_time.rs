//! Presentation time protocol
//!
//! Allows precise feedback on presentation timing for example for smooth video playback.

#[cfg(feature = "client")]
pub use self::generated::client::api as client;

#[cfg(feature = "server")]
pub use self::generated::server::api as server;

mod generated {
    #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
    #![allow(non_upper_case_globals,non_snake_case,unused_imports)]
    #![allow(missing_docs)]

    #[cfg(feature = "client")]
    pub mod client {
        pub mod interfaces {
            pub use wayland_client::protocol_interfaces::{wl_surface_interface,wl_output_interface};
            include!(concat!(env!("OUT_DIR"), "/presentation-time_interfaces.rs"));
        }

        pub mod api {
            // Imports that need to be available to submodules
            // but should not be in public API.
            // Will be fixable with pub(restricted).
            #[doc(hidden)] pub use wayland_client::{Proxy, Handler, RequestResult, EventQueueHandle};
            #[doc(hidden)] pub use super::interfaces;
            #[doc(hidden)] pub use wayland_client::protocol::{wl_surface, wl_output};
            include!(concat!(env!("OUT_DIR"), "/presentation-time_client_api.rs"));
        }
    }

    #[cfg(feature = "server")]
    pub mod server {
        pub mod interfaces {
            pub use wayland_server::protocol_interfaces::{wl_surface_interface,wl_output_interface};
            include!(concat!(env!("OUT_DIR"), "/presentation-time_interfaces.rs"));
        }

        pub mod api {
            // Imports that need to be available to submodules
            // but should not be in public API.
            // Will be fixable with pub(restricted).
            #[doc(hidden)] pub use wayland_server::{Resource, Handler, EventResult, Client, EventLoopHandle};
            #[doc(hidden)] pub use super::interfaces;
            #[doc(hidden)] pub use wayland_server::protocol::{wl_surface, wl_output};
            include!(concat!(env!("OUT_DIR"), "/presentation-time_server_api.rs"));
        }
    }
}
