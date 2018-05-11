//! Client-side Wayland connector
//!
//! ## Overview
//!
//! This crate provides the interfaces and machinery to safely create
//! client applications for the wayland protocol. It is a rust wrapper
//! around the `libwayland-client.so` C library.
//!
//! The wayland protocol revolves around the creation of various objects
//! and the exchange of messages associated to these objects. The initial
//! object is always the `Display`, that you get at initialization of the
//! connection, exposed by this crate as `Display::connect_to_env()`.
//!
//! ## Protocol and messages handling model
//!
//! The protocol being bi-directional, you can send and receive messages.
//! Sending messages is done via methods of `Proxy<_>` objects, receiving
//! and handling them is done by providing implementations.
//!
//! ### Proxies
//!
//! Wayland protocol objects are represented in this crate by `Proxy<I>`
//! objects, where `I` is a type representing the interface of the considered
//! object. And object's interface (think "class" in an object-oriented context)
//! defines which messages it can send and receive.
//!
//! These proxies are used to send messages to the server (in the wayland context,
//! these are called "requests"). To do so, you need to import the appropriate
//! extension trait adding these methods. For example, to use a `Proxy<WlSurface>`,
//! you need to import `protocol::wl_surface::RequestsTrait` from this crate.
//! It is also possible to directly use the `Proxy::<I>::send(..)` method, but
//! this should only be done carefully: using it improperly can mess the protocol
//! state and cause protocol errors, which are fatal to the connection (the server
//! will kill you).
//!
//! There is not a 1 to 1 mapping between `Proxy<I>` instances and protocol
//! objects. Rather, you can think of `Proxy<I>` as an `Rc`-like handle to a
//! wayland object. Multiple instances of it can exist referring to the same
//! protocol object.
//!
//! Similarly, the lifetimes of the protocol objects and the `Proxy<I>` are
//! not tighly tied. As protocol objects are created and destroyed by protocol
//! messages, it can happen that an object gets destroyed while one or more
//! `Proxy<I>` still refers to it. In such case, these proxies will be disabled
//! and their `alive()` method will start to return `false`. Trying to send messages
//! with them will also fail.
//!
//! ### Implementations
//!
//! To receive and process messages from the server to you (in wayland context they are
//! called "events"), you need to provide an `Implementation` for each wayland object
//! created in the protocol session. Whenever a new protocol object is created, you will
//! receive a `NewProxy<I>` object. Providing an implementation via its `implement()` method
//! will turn it into a regular `Proxy<I>` object.
//!
//! **All objects must be implemented**, even if it is an implementation doing nothing.
//! Failure to do so (by dropping the `NewProxy<I>` for example) can cause future fatal
//! errors if the server tries to send an event to this object.
//!
//! An implementation is just a struct implementing the `Implementation<Proxy<I>, I::Event>`
//! trait, where `I` is the interface of the considered object:
//!
//! ```
//! // Example implementation for the wl_surface interface
//! use wayland_client::Proxy;
//! use wayland_client::protocol::wl_surface;
//! use wayland_client::commons::Implementation;
//!
//! struct MyImpl {
//!    // ...
//! }
//!
//! impl Implementation<Proxy<wl_surface::WlSurface>, wl_surface::Event> for MyImpl {
//!     fn receive(&mut self, msg: wl_surface::Event, proxy: Proxy<wl_surface::WlSurface>) {
//!         // process the message...
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! The trait is also automatically implemented for `FnMut(I::Event, Proxy<I>)` closures,
//! so you can use them for simplicity if a full struct would be too cumbersome.
//!
//! ## Event Queues
//!
//! The wayland client machinnery provides the possibility to have one or more event queues
//! handling the processing of received messages. All wayland objects are associated to an
//! event queue, which controls when its events are dispatched.
//!
//! Events received from the server are stored in an internal buffer, and processed (by calling
//! the appropriate implementations) when the associated event queue is dispatched.
//!
//! A default event queue is created at the same time as the initial `Display`, and by default
//! whenever a wayland object is created, it inherits the queue of its parent (the object that sent
//! or receive the message that created the new object). It means that if you only plan to use the
//! default event queue, you don't need to worry about assigning objects to their queues.
//!
//! See the documentation of `EventQueue` for details about dispatching and integrating the event
//! queue into the event loop of your application. See the `Proxy::make_wrapper()` method for
//! details about assigning objects to event queues.
//!
//! ## Dynamic linking with `libwayland-client.so`
//!
//! If you need to gracefully handle the case of a system on which wayland is not installed (by
//! fallbacking to X11 for example), you can do so by activating the `dlopen` cargo feature.
//!
//! When this is done, the library will be loaded a runtime rather than directly linked. And trying
//! to create a `Display` on a system that does not have this library will return a `NoWaylandLib`
//! error.
//!
//! ## Auxiliary libraries
//!
//! Two auxiliary libraries are also available behind cargo features:
//!
//! - the `cursor` feature will try to load `libwayland-cursor.so`, a library helping with loading
//!   system themed cursor textures, to integrate your app in the system theme.
//! - the `egl` feature will try to load `libwayland-egl.so`, a library allowing the creation of
//!   OpenGL surface from wayland surfaces.
//!
//! Both of them will also be loaded at runtime if the `dlopen` feature was provided. See their
//! respective submodules for details about their use.

#![warn(missing_docs)]

#[macro_use]
extern crate bitflags;
extern crate libc;

extern crate wayland_commons;
#[cfg(feature = "native_lib")]
#[macro_use]
extern crate wayland_sys;

mod display;
mod event_queue;
mod globals;
mod proxy;

pub use display::{ConnectError, Display};
pub use event_queue::{EventQueue, QueueToken, ReadEventsGuard};
pub use globals::{GlobalError, GlobalEvent, GlobalManager};
pub use proxy::{NewProxy, Proxy};

#[cfg(feature = "cursor")]
pub mod cursor;

#[cfg(feature = "egl")]
pub mod egl;

/// Re-export of wayland-commons
///
/// Common traits and functions to work with wayland objects
pub mod commons {
    pub use wayland_commons::*;
}

#[cfg(feature = "native_lib")]
/// C-associated types
///
/// Required for plugging wayland-scanner generated protocols
/// or interfacing with C code using wayland objects.
pub mod sys {
    pub use super::generated::c_interfaces as protocol_interfaces;
    pub use wayland_sys::{client, common};
}

/// Generated interfaces for the core wayland protocol
pub mod protocol {
    #[cfg(feature = "native_lib")]
    pub use generated::c_api::*;
}

mod generated {
    #![allow(dead_code, non_camel_case_types, unused_unsafe, unused_variables)]
    #![allow(non_upper_case_globals, non_snake_case, unused_imports)]
    #![allow(missing_docs)]

    #[cfg(feature = "native_lib")]
    pub mod c_interfaces {
        include!(concat!(env!("OUT_DIR"), "/wayland_c_interfaces.rs"));
    }
    #[cfg(feature = "native_lib")]
    pub mod c_api {
        pub(crate) use wayland_commons::{AnonymousObject, Interface, MessageGroup};
        pub(crate) use wayland_sys as sys;
        pub(crate) use {NewProxy, Proxy};
        include!(concat!(env!("OUT_DIR"), "/wayland_c_api.rs"));
    }
}
