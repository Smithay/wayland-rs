//! Server-side Wayland connector
//!
//! ## Overview
//!
//! This crate provides the interfaces and machinery to safely create servers
//! for the Wayland protocol. It can be used as either a rust implementatin of the protocol,
//! or as a wrapper around the system-wide `libwayland-server.so` if you need interoperability
//! with other libraries. This last case is activated by the `use_system_lib` cargo feature.
//!
//! The Wayland protocol revolves around the creation of various objects and the exchange
//! of messages associated to these objects. Whenever a client connects, a `Display` object
//! is automatically created in their object space, which they use as a root to create new
//! objects and bootstrap their state.
//!
//! ## Protocol and messages handling model
//!
//! The protocol being bi-directional, you can send and receive messages.
//! Sending messages is done via methods of Rust objects corresponding to the wayland protocol
//! objects, receiving and handling them is done by providing callbacks.
//!
//! ### Resources
//!
//! The protocol and message model is very similar to the one of `wayland-client`, with the
//! main difference being that the underlying handles to objects are represented by the `Resource<I>`
//! type, very similarly to proxies in `wayland-client`.
//!
//! These resources are used to send messages to the client (in the Wayland context,
//! these are called "events"). You usually don't use them directly, and instead call
//! methods on the Rust objects themselves, which invoke the appropriate `Resource` methods.
//! It is also possible to directly use the `Resource::<I>::send(..)` method.
//!
//! There is not a 1 to 1 mapping between Rust object instances and protocol
//! objects. Rather, you can think of the Rust objects as `Rc`-like handles to a
//! Wayland object. Multiple instances of a Rust object can exist referring to the same
//! protocol object.
//!
//! Similarly, the lifetimes of the protocol objects and the Rust objects are
//! not tightly tied. As protocol objects are created and destroyed by protocol
//! messages, it can happen that an object gets destroyed while one or more
//! Rust objects still refer to it. In such case, these Rust objects will be disabled
//! and the `alive()` method on the underlying `Resource<I>` will start to return `false`.
//! Events that are subsequently sent to them are ignored.
//!
//! ### Filters
//!
//! Your wayland objects can receive requests from the client, which need to be processed.
//! To do so, you can assign `Filter`s to your object. These are specially wrapped closure
//! so that several objects can be assigned to the same `Filter`, to ease state sharing
//! between the code handling different objects.
//!
//! **All objects must be assigned to a filter**, even if it is for doing nothing.
//! Failure to do will cause a `panic!()` if a request is dispatched to the faulty object.
//!
//! A Rust object passed to your implementation is guaranteed to be alive (as it just received
//! a request), unless the exact message received is a destructor (which is indicated in the API
//! documentations).
//!
//! ## General structure
//!
//! The core of your server is the `Display` object. It represent the ability of your program to
//! process Wayland messages. Once this object is created, you can configure it to listen on one
//! or more sockets for incoming client connections (see the `Display` docs for details).
//!
//! `wayland-server` does not include an event loop, and you are expected to drive the wayland socket
//! yourself using the `Display::flush_clients` and `Display::dispatch` methods. The `Display::get_poll_fd`
//! methods provides you with a file descriptor that can be used in a polling structure to integrate
//! the wayland socket in an event loop.

#![warn(missing_docs, missing_debug_implementations)]

#[macro_use]
extern crate bitflags;

#[cfg_attr(feature = "use_system_lib", macro_use)]
extern crate wayland_sys;

mod client;
mod display;
mod globals;
mod resource;

pub use client::Client;
pub use display::Display;
pub use globals::Global;
pub use resource::{Main, Resource};

pub use anonymous_object::AnonymousObject;
pub use wayland_commons::user_data::UserDataMap;
pub use wayland_commons::{
    filter::{DispatchData, Filter},
    Interface, MessageGroup, NoMessage,
};

/// C-associated types
///
/// Required for plugging wayland-scanner generated protocols
/// or interfacing with C code using wayland objects.
pub mod sys {
    pub use wayland_sys::{common, server};
}

// rust implementation
#[cfg(not(feature = "use_system_lib"))]
#[path = "rust_imp/mod.rs"]
mod imp;
// C-lib based implementation
#[cfg(feature = "use_system_lib")]
#[path = "native_lib/mod.rs"]
mod imp;

pub use imp::ResourceMap;

/// Generated interfaces for the core wayland protocol
pub mod protocol {
    #![allow(dead_code, non_camel_case_types, unused_unsafe, unused_variables)]
    #![allow(non_upper_case_globals, non_snake_case, unused_imports)]
    #![allow(missing_docs, clippy::all)]

    pub(crate) use crate::{AnonymousObject, Main, Resource, ResourceMap};
    pub(crate) use wayland_commons::map::{Object, ObjectMetadata};
    pub(crate) use wayland_commons::smallvec;
    pub(crate) use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc};
    pub(crate) use wayland_commons::{Interface, MessageGroup};
    pub(crate) use wayland_sys as sys;
    include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
}

mod anonymous_object {
    use super::{Interface, NoMessage, Resource};
    use std::fmt::{self, Debug, Formatter};

    /// Anonymous interface
    ///
    /// A special Interface implementation representing an
    /// handle to an object for which the interface is not known.
    #[derive(Clone, Eq, PartialEq)]
    pub struct AnonymousObject(Resource<AnonymousObject>);

    impl Interface for AnonymousObject {
        type Request = NoMessage;
        type Event = NoMessage;
        const NAME: &'static str = "<anonymous>";
        const VERSION: u32 = 0;
        fn c_interface() -> *const ::wayland_sys::common::wl_interface {
            ::std::ptr::null()
        }
    }

    impl AsRef<Resource<AnonymousObject>> for AnonymousObject {
        #[inline]
        fn as_ref(&self) -> &Resource<Self> {
            &self.0
        }
    }
    impl From<Resource<AnonymousObject>> for AnonymousObject {
        #[inline]
        fn from(resource: Resource<Self>) -> Self {
            AnonymousObject(resource)
        }
    }
    impl From<AnonymousObject> for Resource<AnonymousObject> {
        #[inline]
        fn from(value: AnonymousObject) -> Self {
            value.0
        }
    }
    impl Debug for AnonymousObject {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_fmt(format_args!("{:?}", self.0))
        }
    }
}

/// Generate an enum joining several objects requests
///
/// This macro allows you to easily create a enum type for use with your message Filters. It is
/// used like so:
///
/// ```no_run
/// # use wayland_server::protocol::{wl_surface::WlSurface, wl_keyboard::WlKeyboard, wl_pointer::WlPointer};
/// # use wayland_server::request_enum;
/// request_enum!(
///     MyEnum |
///     Pointer => WlPointer,
///     Keyboard => WlKeyboard,
///     Surface => WlSurface
/// );
/// ```
///
/// This will generate the following enum, unifying the requests from each of the provided interface:
///
/// ```ignore
/// pub enum MyEnum {
///     Pointer { request: WlPointer::Request, object: Main<WlPointer> },
///     Keyboard { request: WlKeyboard::Request, object: Main<WlKeyboard> },
///     Surface { request: WlSurface::Request, object: Main<WlSurface> }
/// }
/// ```
///
/// It will also generate the appropriate `From<_>` implementation so that a `Filter<MyEnum>` can be
/// used as assignation target for `WlPointer`, `WlKeyboard` and `WlSurface`.
///
/// If you want to add custom messages to the enum, the macro also supports it:
///
/// ```no_run
/// # use wayland_server::protocol::{wl_surface::WlSurface, wl_keyboard::WlKeyboard, wl_pointer::WlPointer};
/// # use wayland_server::request_enum;
/// # struct SomeType;
/// # struct OtherType;
/// request_enum!(
///     MyEnum |
///     Pointer => WlPointer,
///     Keyboard => WlKeyboard,
///     Surface => WlSurface |
///     MyMessage => SomeType,
///     OtherMessage => OtherType
/// );
/// ```
///
/// will generate the following enum:
///
/// ```ignore
/// pub enum MyEnum {
///     Pointer { request: WlPointer::Request, object: Main<WlPointer> },
///     Keyboard { request: WlKeyboard::Request, object: Main<WlKeyboard> },
///     Surface { request: WlSurface::Request, object: Main<WlSurface> },
///     MyMessage(SomeType),
///     OtherMessage(OtherType)
/// }
/// ```
///
/// as well as implementations of `From<SomeType>` and `From<OtherType>`, so that these types can
/// directly be provided into a `Filter<MyEnum>`.

#[macro_export]
macro_rules! request_enum(
    ($(#[$attrs:meta])* $enu:ident | $($evt_name:ident => $iface:ty),*) => {
        $crate::request_enum!($(#[$attrs])* $enu | $($evt_name => $iface),* | );
    };
    ($(#[$attrs:meta])* $enu:ident | $($evt_name:ident => $iface:ty),* | $($name:ident => $value:ty),*) => {
        $(#[$attrs])*
        pub enum $enu {
            $(
                $evt_name { request: <$iface as $crate::Interface>::Request, object: $crate::Main<$iface> },
            )*
            $(
                $name($value),
            )*
        }

        $(
            impl From<($crate::Main<$iface>, <$iface as $crate::Interface>::Request)> for $enu {
                fn from((object, request): ($crate::Main<$iface>, <$iface as $crate::Interface>::Request)) -> $enu {
                    $enu::$evt_name { request, object }
                }
            }
        )*

        $(
            impl From<$value> for $enu {
                fn from(value: $value) -> $enu {
                    $enu::$name(value)
                }
            }
        )*
    };
);
