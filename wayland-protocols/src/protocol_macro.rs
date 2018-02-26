#[macro_escape]
macro_rules! wayland_protocol(
    ($name: expr, [$(($import: ident, $interface: ident)),*]) => {
        #[cfg(all(feature = "client", feature="native_lib"))]
        pub use self::generated::client::c_api as client;

        #[cfg(all(feature = "server", feature="native_lib"))]
        pub use self::generated::server::c_api as server;

        #[cfg(feature = "native_lib")]
        mod generated {
            #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
            #![allow(non_upper_case_globals,non_snake_case,unused_imports)]
            #![allow(missing_docs)]

            #[cfg(feature = "client")]
            pub mod client {
                pub mod c_interfaces {
                    pub use wayland_client::sys::protocol_interfaces::{$($interface),*};
                    include!(concat!(env!("OUT_DIR"), "/", $name, "_c_interfaces.rs"));
                }

                /// Client-side API of this protocol
                pub mod c_api {
                    pub(crate) use wayland_client::{NewProxy, Proxy};
                    pub(crate) use wayland_commons::{AnonymousObject, Interface, MessageGroup};
                    pub(crate) use wayland_sys as sys;
                    pub(crate) use wayland_client::protocol::{$($import),*};
                    include!(concat!(env!("OUT_DIR"), "/", $name, "_c_client_api.rs"));
                }
            }

            #[cfg(feature = "server")]
            pub mod server {
                pub mod c_interfaces {
                    pub use wayland_server::sys::protocol_interfaces::{$($interface),*};
                    include!(concat!(env!("OUT_DIR"), "/", $name, "_c_interfaces.rs"));
                }

                /// Server-side API of this protocol
                pub mod c_api {
                    pub(crate) use wayland_server::{NewResource, Resource};
                    pub(crate) use wayland_commons::{AnonymousObject, Interface, MessageGroup};
                    pub(crate) use wayland_sys as sys;
                    pub(crate) use wayland_server::protocol::{$($import),*};
                    include!(concat!(env!("OUT_DIR"), "/", $name, "_c_server_api.rs"));
                }
            }
        }
    }
);

#[macro_escape]
macro_rules! wayland_protocol_versioned(
    ($name: expr, [$($version: ident),*], $rest:tt) => {
        $(
            #[allow(missing_docs)]
            pub mod $version {
                wayland_protocol!(concat!($name, "-", stringify!($version)), $rest);
            }
        )*
    }
);
