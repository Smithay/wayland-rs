macro_rules! wayland_protocol(
    (
        $name: expr,
        [$(($import: ident, $interface: ident)),*],
        // Path declaration in prot_name is to allow importing from another class of protocols, such as unstable
        [$(($($prot_name:ident)::+, $prot_import: ident, $prot_iface: ident)),*]
    ) => {
        #[cfg(feature = "client")]
        pub use self::generated::client;

        #[cfg(feature = "server")]
        pub use self::generated::server;

        mod generated {
            #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
            #![allow(non_upper_case_globals,non_snake_case,unused_imports)]
            #![allow(missing_docs, clippy::all)]

            #[cfg(feature = "client")]
            pub mod client {
                //! Client-side API of this protocol
                pub(crate) use wayland_client::{Main, Attached, Proxy, ProxyMap, AnonymousObject};
                pub(crate) use wayland_commons::map::{Object, ObjectMetadata};
                pub(crate) use wayland_commons::{Interface, MessageGroup};
                pub(crate) use wayland_commons::wire::{Argument, MessageDesc, ArgumentType, Message};
                pub(crate) use wayland_commons::smallvec;
                pub(crate) use wayland_client::protocol::{$($import),*};
                pub(crate) use wayland_client::sys;
                $(
                    pub(crate) use crate::$($prot_name ::)*client::$prot_import;
                )*
                include!(concat!(env!("OUT_DIR"), "/", $name, "_client_api.rs"));
            }

            #[cfg(feature = "server")]
            pub mod server {
                //! Server-side API of this protocol
                pub(crate) use wayland_server::{Main, AnonymousObject, Resource, ResourceMap};
                pub(crate) use wayland_commons::map::{Object, ObjectMetadata};
                pub(crate) use wayland_commons::{Interface, MessageGroup};
                pub(crate) use wayland_commons::wire::{Argument, MessageDesc, ArgumentType, Message};
                pub(crate) use wayland_commons::smallvec;
                pub(crate) use wayland_server::protocol::{$($import),*};
                pub(crate) use wayland_server::sys;
                $(
                    pub(crate) use crate::$($prot_name ::)*server::$prot_import;
                )*
                include!(concat!(env!("OUT_DIR"), "/", $name, "_server_api.rs"));
            }
        }
    }
);

#[cfg(any(feature = "staging_protocols", feature = "unstable_protocols"))]
macro_rules! wayland_protocol_versioned(
    ($name: expr, [$($version: ident),*], $std_imports:tt, $prot_imports:tt) => {
        $(
            #[allow(missing_docs)]
            pub mod $version {
                wayland_protocol!(concat!($name, "-", stringify!($version)), $std_imports, $prot_imports);
            }
        )*
    }
);
