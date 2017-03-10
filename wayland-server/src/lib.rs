//! Server-side Wayland connector
//!
//! # Overview
//!
//! Setting up the listening socket is done by the `create_display`
//! function, providing you a `Display` object and an `EventLoop`.
//!
//! On the event loop, you'll be able to register the globals
//! you want to advertize, as well as handlers for all ressources
//! created by the clients.
//!
//! You then integrate the wayland event loop in your main event
//! loop to run your compositor.
//!
//! # Handlers and event loop
//!
//! This crate mirrors the callback-oriented design of the
//! Wayland C library by using handler structs: each wayland
//! type defines a `Handler` trait in its module, which one
//! method for each possible requests this object can receive.
//!
//! To use it, you need to build a struct (or enum) that will
//! implement all the traits for all the requests you are interested
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
//!     fn a_request(&mut self,
//!                  evlh: &mut EventLoopHandle,
//!                  client: &Client,
//!                  me: &wl_foo::WlFoo,
//!                  arg1, arg2, // the actual args of the request
//!     ) {
//!         /* handle the request */
//!     }
//! }
//!
//! // declare the handler capability
//! // this boring step is necessary because Rust's type system is
//! // not yet magical enough
//! declare_handler!(MyHandler, wl_foo::Handler, wl_foo::WlFoo);
//! ```
//!
//! ## Event Loop and handlers
//!
//! In your initialization code, you'll need to instantiate
//! your handler and give it to the event queue:
//!
//! ```ignore
//! let handler_id = event_loop.add_handler(MyHandler::new());
//! ```
//!
//! Then, you can register your wayland objects to this handler:
//!
//! ```ignore
//! // This type info is necessary for safety, as at registration
//! // time the event_loop will check that the handler you
//! // specified using handler_id has the same type as provided
//! // as argument, and that this type implements the appropriate
//! // handler trait.
//! event_loop.register::<_, MyHandler>(&my_object, handler_id);
//! ```
//!
//! You can have several handlers in the same event loop,
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
//! from within a handler method, using the `&EventLoopHandle`
//! argument.
//!
//! ## Globals declaration
//!
//! Declaring a global is quite similar to declaring a handler
//! for a ressource. But this time via the `GlobalHandler<R>` trait.
//!
//! This trait declares a single method, `bind`, which is called
//! whenever a client binds this global, providing you with
//! a ressource object representing the newly-created global.
//!
//! You can do what you please with this object, storing it for
//! later if you'll need to send it events, or just let it go
//! go out of scope after having registered it to the appropriate
//! handler.
//!
//! To register globals on the event loop, you need the
//! `register_global` method:
//!
//! ```ignore
//! struct MyHandler { /* ... */ }
//!
//! // creating a handler for the global wl_foo
//! impl GlobalHandler<WlFoo> for MyHandler {
//!     fn bind(&mut self,
//!             evlh: &mut EventLoopHandle,
//!             client: &Client,
//!             global: WlFoo
//!     ) {
//!         /* do something with it */
//!     }
//! }
//!
//! /* ... */
//!
//! // then somewhere in the initialization code
//! let handler_id = event_loop.add_handler(MyHandler::new());
//! // bind this handler to a global for interface wl_foo version 3
//! // specifying the types is mandatory, and compability of this
//! // handler with the global is checked at registration time.
//! let foo = event_loop.register_global::<WlFoo, MyHandler>(handler_is, 3);
//! // foo contains a handle to the global, that you can use later
//! // to destroy it:
//! foo.destroy();
//! // if you don't plan to ever destroy this global, you can ignore
//! // this return value, letting it out of scope will not destroy
//! // the global.
//! ```
//!
//! ## Event loop integration
//!
//! Once the setup phase is done, you can integrate the
//! event loop in the main event loop of your program.
//!
//! Either all you need is for it to run indefinitely (external
//! events are checked in an other thread?):
//!
//! ```ignore
//! event_loop.run();
//! ```
//!
//! Or you can integrate it with more control:
//!
//! ```ignore
//! loop {
//!     // flush events to client sockets
//!     display.flush_clients();
//!     // receive request from clients and dispatch them
//!     // blocking if no request is pending for at most
//!     // 10ms
//!     event_loop.dispatch(Some(10)).unwrap();
//!     // then you can check events from other sources if
//!     // you need to
//! }
//! ```
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

#[macro_use] extern crate bitflags;
#[macro_use] extern crate wayland_sys;
extern crate libc;
extern crate nix;

pub use generated::server as protocol;
pub use generated::interfaces as protocol_interfaces;

pub use client::Client;
pub use display::{Display, create_display};
pub use event_loop::{EventLoop, EventLoopHandle, StateGuard, Global, GlobalHandler, Init, Destroy,
                     resource_is_registered};

use wayland_sys::server::*;
use wayland_sys::common::{wl_interface, wl_argument};

mod client;
mod display;
mod event_loop;
mod event_sources;

pub mod sources {
    //! Secondary event sources
    //!
    //! This module contains the types & traits to work with
    //! different kind of event sources that can be registered to and
    //! event loop, other than the wayland protocol sockets.
    pub use ::event_sources::{FdEventSource, FdEventSourceHandler};
    pub use ::event_sources::{FdInterest, READ, WRITE};
    pub use ::event_sources::{TimerEventSource, TimerEventSourceHandler};
    pub use ::event_sources::{SignalEventSource, SignalEventSourceHandler};
}

/// Common routines for wayland resource objects.
///
/// All wayland objects automatically implement this trait
/// as generated by the scanner.
///
/// It is mostly used for internal use by the library, and you
/// should only need these methods for interfacing with C library
/// working on wayland objects.
pub trait Resource {
    /// Pointer to the underlying wayland proxy object
    fn ptr(&self) -> *mut wl_resource;
    /// Create an instance from a wayland pointer
    ///
    /// The pointer must refer to a valid wayland resource
    /// of the appropriate interface, but that have not yet
    /// been seen by the library.
    ///
    /// The library will take control of the object (notably
    /// overwrite its user_data).
    unsafe fn from_ptr_new(*mut wl_resource) -> Self;
    /// Create an instance from a wayland pointer
    ///
    /// The pointer must refer to a valid wayland resource
    /// of the appropriate interface, and have already been
    /// initialized by the library (it'll assume this proxy
    /// user_data contains a certain kind of data).
    unsafe fn from_ptr_initialized(*mut wl_resource) -> Self;
    /// Pointer to the interface representation
    fn interface_ptr() -> *const wl_interface;
    /// Internal wayland name of this interface
    fn interface_name() -> &'static str;
    /// Max version of this interface supported
    fn supported_version() -> u32;
    /// Current version of the interface this resource is instantiated with
    fn version(&self) -> i32;
    /// Check if the resource behind this handle is actually still alive
    fn is_alive(&self) -> bool;
    /// Check of two handles are actually the same wayland object
    ///
    /// Returns `false` if any of the objects has already been destroyed
    fn equals(&self, &Self) -> bool;
    /// Set a pointer associated as user data on this resource
    ///
    /// All handles to the same wayland object share the same user data pointer.
    ///
    /// The get/set operations are atomic, no more guarantee is given. If you need
    /// to synchronise access to this data, it is your responsibility to add a Mutex
    /// or any other similar mechanism.
    fn set_user_data(&self, ptr: *mut ());
    /// Get the pointer associated as user data on this resource
    ///
    /// All handles to the same wayland object share the same user data pointer.
    ///
    /// See `set_user_data` for synchronisation guarantee.
    fn get_user_data(&self) -> *mut ();
    /// Posts a protocol error to this resource
    ///
    /// The error code can be obtained from the various `Error` enums of the protocols.
    ///
    /// An error is fatal to the client that caused it.
    fn post_error(&self, error_code: u32, msg: String) {
        // If `str` contains an interior null, the actuall transmitted message will
        // be truncated at this point.
        unsafe {
            let cstring = ::std::ffi::CString::from_vec_unchecked(msg.into());
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_resource_post_error,
                self.ptr(),
                error_code,
                cstring.as_ptr()
            )
        }
    }
}

/// Possible outcome of the call of a event on a resource
pub enum EventResult<T> {
    /// Message has been buffered and will be sent to client
    Sent(T),
    /// This resource is already destroyed, request has been ignored
    Destroyed
}

impl<T> EventResult<T> {
    /// Assert that result is successfull and extract the value.
    ///
    /// Panics with provided error message if the result was `Destroyed`.
    pub fn expect(self, error: &str) -> T {
        match self {
            EventResult::Sent(v) => v,
            EventResult::Destroyed => panic!("{}", error)
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
pub unsafe trait Handler<T: Resource> {
    /// Dispatch a message.
    unsafe fn message(&mut self, evq: &mut EventLoopHandle, client: &Client, resource: &T, opcode: u32, args: *const wl_argument) -> Result<(),()>;
}

mod generated {
    #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
    #![allow(non_upper_case_globals,non_snake_case,unused_imports)]
    #![allow(missing_docs)]

    pub mod interfaces {
        //! Interfaces for the core protocol
        //!
        //! You might need them for the bindings generated for protocol extensions
        include!(concat!(env!("OUT_DIR"), "/wayland_interfaces.rs"));
    }

    pub mod server {
        //! The wayland core protocol
        //!
        //! This module contains all objects of the core wayland protocol.
        //!
        //! It has been generated from the `wayland.xml` protocol file
        //! using `wayland_scanner`.
        // Imports that need to be available to submodules
        // but should not be in public API.
        // Will be fixable with pub(restricted).
        #[doc(hidden)] pub use {Resource, EventLoopHandle, Handler, Client, EventResult};
        #[doc(hidden)] pub use super::interfaces;

        include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
    }
}

pub mod sys {
    //! Reexports of types and objects from wayland-sys
    pub use wayland_sys::server::*;
    pub use wayland_sys::common::*;
}
