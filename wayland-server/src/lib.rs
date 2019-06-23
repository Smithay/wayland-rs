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
#[cfg(not(feature = "native_lib"))]
#[macro_use]
extern crate downcast_rs as downcast;
#[macro_use]
extern crate smallvec;
extern crate libc;
extern crate mio;
extern crate nix;

extern crate wayland_commons;
#[cfg_attr(feature = "native_lib", macro_use)]
extern crate wayland_sys;

mod client;
mod display;
mod globals;
mod resource;

pub use client::Client;
pub use display::Display;
pub use globals::Global;
pub use resource::{HandledBy, NewResource, Resource};

pub use anonymous_object::AnonymousObject;
pub use wayland_commons::utils::UserDataMap;
pub use wayland_commons::{Interface, MessageGroup, NoMessage};

/// C-associated types
///
/// Required for plugging wayland-scanner generated protocols
/// or interfacing with C code using wayland objects.
pub mod sys {
    pub use wayland_sys::{common, server};
}

// rust implementation
#[cfg(not(feature = "native_lib"))]
#[path = "rust_imp/mod.rs"]
mod imp;
// C-lib based implementation
#[cfg(feature = "native_lib")]
#[path = "native_lib/mod.rs"]
mod imp;

pub use imp::ResourceMap;

/// Generated interfaces for the core wayland protocol
pub mod protocol {
    #![allow(dead_code, non_camel_case_types, unused_unsafe, unused_variables)]
    #![allow(non_upper_case_globals, non_snake_case, unused_imports)]
    #![allow(missing_docs)]
    #![cfg_attr(feature = "cargo-clippy", allow(clippy))]

    pub(crate) use wayland_commons::map::{Object, ObjectMetadata};
    pub(crate) use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc};
    pub(crate) use wayland_commons::{Interface, MessageGroup};
    pub(crate) use wayland_sys as sys;
    pub(crate) use {AnonymousObject, HandledBy, NewResource, Resource, ResourceMap};
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
