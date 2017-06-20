//! Client-side Wayland connector
//!
//! # Overview
//!
//! Connection to the Wayland compositor is achieved by
//! the `default_connect()` function, which provides you
//! with a `WlDisplay` and an `EventQueue`.
//!
//! From the display, you'll retrieve the registry, from
//! which you can instantiate the globals you need. This
//! step being really similar in most cases, this crate
//! contains an utility struct `EnvHandler` which can do
//! this job for you. See its documentation for details.
//!
//! You then register your handlers for events to the
//! event queue, and integrate it in your main event loop.
//!
//! # Handlers and event queues
//!
//! This crate mirrors the callback-oriented design of the
//! Wayland C library by using handler structs: each wayland
//! type defines a `Handler` trait in its module, which one
//! method for each possible event this object can receive.
//!
//! To use it, you need to build a struct (or enum) that will
//! implement all the traits for all the events you are interested
//! in. All methods of handler traits provide a default
//! implementation foing nothing, so you don't need to write
//! empty methods for events you want to ignore. You also need
//! to declare the handler capability for your struct using
//! the `declare_handler!(..)` macro. A single struct can be
//! handler for several wayland interfaces at once.
//!
//! ## Example of handler
//!
//! ```ignore
//! /*  writing a handler for an wl_foo interface */
//! // import the module of this interface
//! use wl_foo;
//!
//! struct MyHandler { /* some fields to store state */ }
//!
//! // implement handerl trait:
//! impl wl_foo::Handler for MyHandler {
//!     fn an_event(&mut self,
//!                 evqh: &mut EventQueueHandle,
//!                 me: &wl_foo::WlFoo,
//!                 arg1, arg2, // the actual args of the event
//!     ) {
//!         /* handle the event */
//!     }
//! }
//!
//! // declare the handler capability
//! // this boring step is necessary because Rust's type system is
//! // not yet magical enough
//! declare_handler!(MyHandler, wl_foo::Handler, wl_foo::WlFoo);
//! ```
//!
//! ## Event Queues and handlers
//!
//! In your initialization code, you'll need to instantiate
//! your handler and give it to the event queue:
//!
//! ```ignore
//! let handler_id = event_queue.add_handler(MyHandler::new());
//! ```
//!
//! Then, you can register your wayland objects to this handler:
//!
//! ```ignore
//! // This type info is necessary for safety, as at registration
//! // time the event_queue will check that the handler you
//! // specified using handler_id has the same type as provided
//! // as argument, and that this type implements the appropriate
//! // handler trait.
//! event_queue.register::<_, MyHandler>(&my_object, handler_id);
//! ```
//!
//! You can have several handlers in the same event queue,
//! but they cannot share their state without synchronisation
//! primitives like `Arc`, `Mutex` and friends, so if two handlers
//! need to share some state, you should consider building them
//! as a single struct.
//!
//! A given wayland object can only be registered to a single
//! handler at a given time, re-registering it to a new handler
//! will overwrite the previous configuration.
//!
//! Handlers can be created, and objects registered to them
//! from within a handler method, using the `&EventQueueHandle`
//! argument.
//!
//! ## Event loop integration
//!
//! Once this setup is done, you can integrate the event queue
//! to the main event loop of your program:
//!
//! ```ignore
//! loop {
//!     // flush events to the server
//!     display.flush().unwrap();
//!     // receive events from the server and dispatch them
//!     // to handlers (might block)
//!     event_queue.dispatch().unwrap();
//! }
//! ```
//!
//! For more precise control of the flow of the event queue
//! (and importantly non-blocking options), see `EventQueue`
//! documentation.
//!
//! # Protocols integration
//!
//! This crate provides the basic primitives as well as the
//! core wayland protocol (in the `protocol` module), but
//! other protocols can be integrated from XML descriptions.
//!
//! The the crate `wayland_scanner` and its documentation for
//! details about how to do so.

#![warn(missing_docs)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate wayland_sys;
extern crate libc;

pub use generated::client as protocol;
pub use generated::interfaces as protocol_interfaces;

use wayland_sys::client::wl_proxy;
use wayland_sys::common::{wl_argument, wl_interface};

mod display;
mod event_queue;
mod env;

#[cfg(feature = "egl")]
pub mod egl;

#[cfg(feature = "cursor")]
pub mod cursor;

pub use display::{ConnectError, FatalError, connect_to, default_connect};
pub use env::{EnvHandler, EnvHandlerInner};
pub use event_queue::{EventQueue, EventQueueHandle, Init, ReadEventsGuard, RegisterStatus, StateGuard};

/// Common routines for wayland proxy objects.
///
/// All wayland objects automatically implement this trait
/// as generated by the scanner.
///
/// It is mostly used for internal use by the library, and you
/// should only need these methods for interfacing with C library
/// working on wayland objects.
pub trait Proxy {
    /// Pointer to the underlying wayland proxy object
    fn ptr(&self) -> *mut wl_proxy;
    /// Create an instance from a wayland pointer
    ///
    /// The pointer must refer to a valid wayland proxy
    /// of the appropriate interface, but that have not
    /// yet been seen by the library.
    ///
    /// The library will take control of the object (notably
    /// overwrite its user_data).
    unsafe fn from_ptr_new(*mut wl_proxy) -> Self;
    /// Create an instance from a wayland pointer
    ///
    /// The pointer must refer to a valid wayland proxy
    /// of the appropriate interface. The library will detect if the
    /// proxy is already managed by it or not. If it is not, this
    /// proxy will be considered as "unmanaged", and should then
    /// be handled with care.
    unsafe fn from_ptr_initialized(*mut wl_proxy) -> Self;
    /// Pointer to the interface representation
    fn interface_ptr() -> *const wl_interface;
    /// Internal wayland name of this interface
    fn interface_name() -> &'static str;
    /// Max version of this interface supported
    fn supported_version() -> u32;
    /// Current version of the interface this proxy is instantiated with
    fn version(&self) -> u32;
    /// Check if the proxy behind this handle is actually still alive
    fn status(&self) -> Liveness;
    /// Check of two handles are actually the same wayland object
    ///
    /// Returns `false` if any of the objects has already been destroyed
    fn equals(&self, &Self) -> bool;
    /// Set a pointer associated as user data on this proxy
    ///
    /// All proxies to the same wayland object share the same user data pointer.
    ///
    /// The get/set operations are atomic, no more guarantee is given. If you need
    /// to synchronise access to this data, it is your responsibility to add a Mutex
    /// or any other similar mechanism.
    ///
    /// If this proxy is not managed by wayland-client, this does nothing.
    fn set_user_data(&self, ptr: *mut ());
    /// Get the pointer associated as user data on this proxy
    ///
    /// All proxies to the same wayland object share the same user data pointer.
    ///
    /// See `set_user_data` for synchronisation guarantee.
    ///
    /// If this proxy is not managed by wayland-client, this returns a null pointer.
    fn get_user_data(&self) -> *mut ();
    /// Clone this proxy handle
    ///
    /// Will only succeed if the proxy is managed by this library and
    /// is still alive.
    fn clone(&self) -> Option<Self>
    where
        Self: Sized,
    {
        if self.status() == Liveness::Alive {
            Some(unsafe { self.clone_unchecked() })
        } else {
            None
        }

    }
    /// Unsafely clone this proxy handle
    ///
    /// This function is unsafe because if the proxy is unmanaged, the lib
    /// has no knowledge of its lifetime, and cannot ensure that the new handle
    /// will not outlive the object.
    unsafe fn clone_unchecked(&self) -> Self
    where
        Self: Sized,
    {
        // TODO: this can be more optimized with codegen help, but would be a
        // breaking change, so do it at next breaking release
        Self::from_ptr_initialized(self.ptr())
    }
}

/// Possible outcome of the call of a request on a proxy
#[derive(Debug)]
pub enum RequestResult<T> {
    /// Message has been buffered and will be sent to server
    Sent(T),
    /// This proxy is already destroyed, request has been ignored
    Destroyed,
}

impl<T> RequestResult<T> {
    /// Assert that result is successfull and extract the value.
    ///
    /// Panics with provided error message if the result was `Destroyed`.
    pub fn expect(self, error: &str) -> T {
        match self {
            RequestResult::Sent(v) => v,
            RequestResult::Destroyed => panic!("{}", error),
        }
    }
}

/// Generic handler trait
///
/// This trait is automatically implemented for objects that implement
/// the appropriate interface-specific `Handler` traits. It represents
/// the hability for a type to handle events directed to a given wayland
/// interface.
///
/// For example, implementing `wl_surface::Handler` for you type will
/// automatically provide it with an implementation of
/// `Handler<WlSurface>` as well. This is the only correct way
/// to implement this trait, and you should not attempt to implement it
/// yourself.
pub unsafe trait Handler<T: Proxy> {
    /// Dispatch a message.
    unsafe fn message(&mut self, evq: &mut EventQueueHandle, proxy: &T, opcode: u32,
                      args: *const wl_argument)
                      -> Result<(), ()>;
}

/// Represents the state of liveness of a wayland object
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Liveness {
    /// This object is alive and its requests can be called
    Alive,
    /// This object is dead, calling its requests will do nothing and
    /// return and error.
    Dead,
    /// This object is not managed by `wayland-client`, you can call its methods
    /// but this might crash the program if it was actually dead.
    Unmanaged,
}

mod generated {
    #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
    #![allow(non_upper_case_globals,non_snake_case,unused_imports)]
    #![allow(missing_docs)]

    pub mod interfaces {
        //! Interfaces for the core protocol
        // You might need them for the bindings generated for protocol extensions
        include!(concat!(env!("OUT_DIR"), "/wayland_interfaces.rs"));
    }

    pub mod client {
        //! The wayland core protocol
        // This module contains all objects of the core wayland protocol.
        //
        // It has been generated from the `wayland.xml` protocol file
        // using `wayland_scanner`.

        // Imports that need to be available to submodules
        // but should not be in public API.
        // Will be fixable with pub(restricted).

        #[doc(hidden)]
        pub use super::interfaces;
        #[doc(hidden)]
        pub use {Handler, Liveness, Proxy, RequestResult};
        #[doc(hidden)]
        pub use event_queue::EventQueueHandle;
        include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
    }
}

pub mod sys {
    //! Reexports of types and objects from wayland-sys

    pub use wayland_sys::client::*;
    pub use wayland_sys::common::*;
}
