//! Server-side Wayland connector
//!
//! ## Overview
//!
//! This crate provides the interfaces and machinery to safely create servers
//! for the Wayland protocol. It is a rust wrapper around the `libwayland-server.so`
//! C library.
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
//! objects, receiving and handling them is done by providing implementations.
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
//! ### Implementations
//!
//! To receive and process messages from the clients to you (in Wayland context they are
//! called "requests"), you need to provide an `Implementation` for each Wayland object
//! created in the protocol session. Whenever a new protocol object is created, you will
//! receive a `NewResource<I>` object. Providing an implementation via its `implement()` method
//! will turn it into a regular Rust object.
//!
//! **All objects must be implemented**, even if it is an implementation doing nothing.
//! Failure to do so (by dropping the `NewResource<I>` for example) can cause future fatal
//! protocol errors if the client tries to send a request to this object.
//!
//! An implementation is a struct implementing the `RequestHandler` trait for the interface
//! of the considered object. Alternatively, an `FnMut(I::Request, I)` closure can be
//! used with the `implement_closure()` method, where `I` is the interface
//! of the considered object.
//!
//! A Rust object passed to your implementation is guaranteed to be alive (as it just received
//! a request), unless the exact message received is a destructor (which is indicated in the API
//! documentations).
//!
//! ## Event loops and general structure
//!
//! The core of your server is the `Display` object. It represent the ability of your program to
//! process Wayland messages. Once this object is created, you can configure it to listen on one
//! or more sockets for incoming client connections (see the `Display` docs for details).
//!
//! To properly function, this Wayland implementation also needs an event loop structure,
//! which is here provided by the `calloop` crate. It is a public dependency and is reexported
//! as `wayland_server::calloop`.

#![warn(missing_docs)]

#[macro_use]
extern crate bitflags;
pub extern crate calloop;
#[cfg(not(feature = "use_system_lib"))]
#[macro_use]
extern crate downcast_rs as downcast;
extern crate libc;
extern crate mio;
extern crate nix;

extern crate wayland_commons;
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
pub use wayland_commons::{filter::Filter, Interface, MessageGroup, NoMessage};

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
    pub(crate) use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc};
    pub(crate) use wayland_commons::{Interface, MessageGroup};
    pub(crate) use wayland_sys as sys;
    include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
}

mod anonymous_object {
    use super::{Interface, NoMessage, Resource};

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
}

/*
 * A raw Fd Evented struct
 */

pub(crate) struct Fd(pub ::std::os::unix::io::RawFd);

impl ::mio::Evented for Fd {
    fn register(
        &self,
        poll: &::mio::Poll,
        token: ::mio::Token,
        interest: ::mio::Ready,
        opts: ::mio::PollOpt,
    ) -> ::std::io::Result<()> {
        ::mio::unix::EventedFd(&self.0).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &::mio::Poll,
        token: ::mio::Token,
        interest: ::mio::Ready,
        opts: ::mio::PollOpt,
    ) -> ::std::io::Result<()> {
        ::mio::unix::EventedFd(&self.0).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &::mio::Poll) -> ::std::io::Result<()> {
        ::mio::unix::EventedFd(&self.0).deregister(poll)
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
    ($enu:ident | $($evt_name:ident => $iface:ty),*) => {
        $crate::request_enum!($enu | $($evt_name => $iface),* | );
    };
    ($enu:ident | $($evt_name:ident => $iface:ty),* | $($name:ident => $value:ty),*) => {
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
