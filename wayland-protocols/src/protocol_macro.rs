#[macro_escape]
macro_rules! wayland_protocol(
    ($name: expr, $(($import: ident, $interface: ident)),*) => {
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
                    pub use wayland_client::protocol_interfaces::{$($interface),*};
                    include!(concat!(env!("OUT_DIR"), "/", $name, "_interfaces.rs"));
                }

                pub mod api {
                    // Imports that need to be available to submodules
                    // but should not be in public API.
                    // Will be fixable with pub(restricted).
                    #[doc(hidden)] pub use wayland_client::{Proxy, Handler, RequestResult, EventQueueHandle, Liveness};
                    #[doc(hidden)] pub use super::interfaces;
                    #[doc(hidden)] pub use wayland_client::protocol::{$($import),*};
                    include!(concat!(env!("OUT_DIR"), "/", $name, "_client_api.rs"));
                }
            }

            #[cfg(feature = "server")]
            pub mod server {
                pub mod interfaces {
                    pub use wayland_server::protocol_interfaces::{$($interface),*};
                    include!(concat!(env!("OUT_DIR"), "/", $name, "_interfaces.rs"));
                }

                pub mod api {
                    // Imports that need to be available to submodules
                    // but should not be in public API.
                    // Will be fixable with pub(restricted).
                    #[doc(hidden)] pub use wayland_server::{Resource, Handler, EventResult, Client, EventLoopHandle, Liveness};
                    #[doc(hidden)] pub use super::interfaces;
                    #[doc(hidden)] pub use wayland_server::protocol::{$($import),*};
                    include!(concat!(env!("OUT_DIR"), "/", $name, "_server_api.rs"));
                }
            }
        }
    }
);
