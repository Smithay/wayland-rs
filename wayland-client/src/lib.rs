//! Interface for interacting with the Wayland protocol, client-side.
//!
//! ## General concepts
//!
//! This crate is structured around four main objects: the [`Connection`] and [`EventQueue`] structs,
//! proxies (objects implementing the [`Proxy`] trait), and the [`Dispatch`] trait.
//!
//! The [`Connection`] is the heart of this crate, it represents you connection to the Wayland server, and
//! you'll generally initialize it using the [`Connection::connect_to_env()`](Connection::connect_to_env)
//! method, which will attempt to open a Wayland connection following the configuration specified by the
//! environment.
//!
//! Once you have a [`Connection`], you can create an [`EventQueue`] from it. This [`EventQueue`] will take
//! care of processing events from the Wayland server and delivering it to you processing logic, in the form
//! of a state struct with several [`Dispatch`] implementations (see below).
//!
//! Each of the Wayland object you can manipulate is represented by a struct implementing the [`Proxy`]
//! trait. Thos structs are automatically generated from the wayland XML protocol specification. This crate
//! provides the types generated from the core protocol in the [`protocol`] module. For other standard
//! protocols, see the `wayland-protocols` crate.
//!
//! ## Event dispatching
//!
//! The core event dispatching logic provided by this crate is build around the [`EventQueue`] struct. In
//! this paradigm, receiving and processing events is a two-step process:
//!
//! - First events are read from the Wayland socket, for each event the backend figures with [`EventQueue`]
//!   manages it, and enqueues the event in an internal buffer of that queue.
//! - Then, the [`EventQueue`] empties its internal buffer by sequentially invoking the appropriate
//!   [`Dispatch::event()`] method on the `State` value that was provided to it.
//!
//! The main goal of this structure is to make your `State` accessible without synchronization to most of
//! your event-processing logic, to reduce the plumbing costs. See [`EventQueue`] documentation for
//! explanations of how to drive your event loop using it and explanation about when and how to use multiple
//! event queues in your app.
//!
//! ### The [`Dispatch`] trait and dispatch delegation
//!
//! In this paradigm, your `State` needs to implement `Dispatch<O, _>` for every Wayland object `O` it needs to
//! process events for. This is ensured by the fact that, whenever creating an object using the methods on
//! an other object, you need to pass a [`QueueHandle<State>`] from the [`EventQueue`] that will be
//! managing the newly created object.
//!
//! However, implementing all those traits on your own is a lot of (often uninteresting) work. To make this
//! easier a composition mechanism is provided using the [`delegate_dispatch!`] macro. This way, another
//! library (such as Smithay's Client Toolkit) can provide generic [`Dispatch`] implementations that you
//! can reuse on your own app by delegating those objects to that provided implementation. See the
//! documentation of those traits and macro for details.
//!
//! ## Getting started example
//!
//! As an overview of how this crate is used, here is a commented example of a program that connects to the
//! Wayland server and lists the globals this server advertized throught the `wl_registry`:
//!
//! ```rust,no_run
//! use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};
//! // This struct represents the state of our app. This simple app does not
//! // need any state, by this type still supports the `Dispatch` implementations.
//! struct AppData;
//!
//! // Implement `Dispatch<WlRegistry, ()> for out state. This provides the logic
//! // to be able to process events for the wl_registry interface.
//! //
//! // The second type parameter is the user-data of our implementation. It is a
//! // mechanism that allows you to associate a value to each particular Wayland
//! // object, and allow different dispatching logic depending on the type of the
//! // associated value.
//! //
//! // In this example, we just use () as we don't have any value to associate. See
//! // the `Dispatch` documentation for more details about this.
//! impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
//!     fn event(
//!         _state: &mut Self,
//!         _: &wl_registry::WlRegistry,
//!         event: wl_registry::Event,
//!         _: &(),
//!         _: &Connection,
//!         _: &QueueHandle<AppData>,
//!     ) {
//!         // When receiving events from the wl_registry, we are only interested in the
//!         // `global` event, which signals a new available global.
//!         // When receiving this event, we just print its characteristics in this example.
//!         if let wl_registry::Event::Global { name, interface, version } = event {
//!             println!("[{}] {} (v{})", name, interface, version);
//!         }
//!     }
//! }
//!
//! // The main function of our program
//! fn main() {
//!     // Create a Wayland connection by connecting to the server through the
//!     // environment-provided configuration.
//!     let conn = Connection::connect_to_env().unwrap();
//!
//!     // Retrieve the WlDisplay Wayland object from the connection. This object is
//!     // the starting point of any Wayland program, from which all other objects will
//!     // be created.
//!     let display = conn.display();
//!
//!     // Create an event queue for our event processing
//!     let mut event_queue = conn.new_event_queue();
//!     // An get its handle to associated new objects to it
//!     let qh = event_queue.handle();
//!
//!     // Create a wl_registry object by sending the wl_display.get_registry request
//!     // This method takes two arguments: a handle to the queue the newly created
//!     // wl_registry will be assigned to, and the user-data that should be associated
//!     // with this registry (here it is () as we don't need user-data).
//!     let _registry = display.get_registry(&qh, ());
//!
//!     // At this point everything is ready, and we just need to wait to receive the events
//!     // from the wl_registry, our callback will print the advertized globals.
//!     println!("Advertized globals:");
//!
//!     // To actually receive the events, we invoke the `sync_roundtrip` method. This method
//!     // is special and you will generally only invoke it during the setup of your program:
//!     // it will block until the server has received and processed all the messages you've
//!     // sent up to now.
//!     //
//!     // In our case, that means it'll block until the server has received our
//!     // wl_display.get_registry request, and as a reaction has sent us a batch of
//!     // wl_registry.global events.
//!     //
//!     // `sync_roundtrip` will then empty the internal buffer of the queue it has been invoked
//!     // on, and thus invoke our `Dispatch` implementation that prints the list of advertized
//!     // globals.
//!     event_queue.roundtrip(&mut AppData).unwrap();
//! }
//! ```
//!
//! ## Advanced use
//!
//! ### Bypassing [`Dispatch`]
//!
//! It may be that for some of your objects, handling them via the [`EventQueue`] is unpractical. For example
//! if processing the events from those objects don't require accessing some global state, and/or you need to
//! handle them in a context where cranking an event loop is unpractical.
//!
//! In those contexts, this crate also provides some escape-hatches to directly interface with the low-level
//! APIs from `wayland-backend`, allowing you to register callbacks for those objects that will be invoked
//! whenever they receive an event and *any* event queue from the program is being dispatched. Those
//! callbacks are more constrained: they don't get a `&mut State` reference, and must be threadsafe. See
//! [`Proxy::send_constructor`] for details about how to assign such callbacks to objects.
//!
//! ### Interaction with FFI
//!
//! It can happen that you'll need to interact with Wayland states accross FFI, a typical example would be if
//! you need to use the [`raw-window-handle`](https://docs.rs/raw-window-handle/) crate.
//!
//! In this case, you'll need to do it in two steps, by explicitly working with `wayland-backend`, adding
//! it to your dependencies and enabling its `client_system` feature.
//!
//! - If you need to send pointers to FFI, you can retrive the `*mut wl_proxy` pointers from the proxies by
//!   first getting the [`ObjectId`](crate::backend::ObjectId) using the [`Proxy::id()`] method, and then
//!   the `ObjectId::as_ptr()` method.
//! - If you need to receive pointers from FFI, you need to first create a
//!   [`Backend`](crate::backend::Backend) from the `*mut wl_display` using the `from_external_display()`
//!   method (see `wayland-backend` docs), and then make it into a [`Connection`] using
//!   [`Connection::from_backend()`]. Similarly, you can make [`ObjectId`]s from the `*mut wl_proxy` pointers
//!   using `ObjectId::from_ptr()`, and then make the proxies using [`Proxy::from_id`].
//!
//! ## Logging
//!
//! This crate can generate some runtime error message (notably when a protocol error occurs). By default
//! those messages are printed to stderr. If you activate the `log` cargo feature, they will instead be
//! piped through the `log` crate.

#![allow(clippy::needless_doctest_main)]
#![warn(missing_docs, missing_debug_implementations)]
#![forbid(improper_ctypes, unsafe_op_in_unsafe_fn)]
#![cfg_attr(coverage, feature(no_coverage))]

use std::{
    hash::{Hash, Hasher},
    os::unix::io::RawFd,
    sync::Arc,
};
use wayland_backend::{
    client::{InvalidId, ObjectData, ObjectId, WaylandError, WeakBackend},
    io_lifetimes::OwnedFd,
    protocol::{Interface, Message},
};

mod conn;
mod event_queue;
pub mod globals;

/// Backend reexports
pub mod backend {
    pub use wayland_backend::client::{
        Backend, InvalidId, NoWaylandLib, ObjectData, ObjectId, ReadEventsGuard, WaylandError,
        WeakBackend,
    };
    pub use wayland_backend::io_lifetimes;
    pub use wayland_backend::protocol;
    pub use wayland_backend::smallvec;
}

pub use wayland_backend::protocol::WEnum;

pub use conn::{ConnectError, Connection};
pub use event_queue::{Dispatch, EventQueue, QueueFreezeGuard, QueueHandle, QueueProxyData};

// internal imports for dispatching logging depending on the `log` feature
#[cfg(feature = "log")]
#[allow(unused_imports)]
use log::{debug as log_debug, error as log_error, info as log_info, warn as log_warn};
#[cfg(not(feature = "log"))]
#[allow(unused_imports)]
use std::{
    eprintln as log_error, eprintln as log_warn, eprintln as log_info, eprintln as log_debug,
};

/// Generated protocol definitions
///
/// This module is automatically generated from the `wayland.xml` protocol specification,
/// and contains the interface definitions for the core Wayland protocol.
#[allow(missing_docs)]
pub mod protocol {
    use self::__interfaces::*;
    use crate as wayland_client;
    pub mod __interfaces {
        wayland_scanner::generate_interfaces!("wayland.xml");
    }
    wayland_scanner::generate_client_code!("wayland.xml");
}

/// Trait representing a Wayland interface
pub trait Proxy: Clone + std::fmt::Debug + Sized {
    /// The event enum for this interface
    type Event;
    /// The request enum for this interface
    type Request;

    /// The interface description
    fn interface() -> &'static Interface;

    /// The ID of this object
    fn id(&self) -> ObjectId;

    /// The version of this object
    fn version(&self) -> u32;

    /// Checks if the Wayland object associated with this proxy is still alive
    fn is_alive(&self) -> bool {
        if let Some(backend) = self.backend().upgrade() {
            backend.info(self.id()).is_ok()
        } else {
            false
        }
    }

    /// Access the user-data associated with this object
    fn data<U: Send + Sync + 'static>(&self) -> Option<&U>;

    /// Access the raw data associated with this object.
    ///
    /// For objects created using the scanner-generated methods, this will be an instance of the
    /// [QueueProxyData] type.
    fn object_data(&self) -> Option<&Arc<dyn ObjectData>>;

    /// Access the backend associated with this object
    fn backend(&self) -> &backend::WeakBackend;

    /// Create an object proxy from its ID
    ///
    /// Returns an error this the provided object ID does not correspond to
    /// the `Self` interface.
    ///
    /// **Note:** This method is mostly meant as an implementation detail to be
    /// used by code generated by wayland-scanner.
    fn from_id(conn: &Connection, id: ObjectId) -> Result<Self, InvalidId>;

    /// Create an inert object proxy
    ///
    /// **Note:** This method is mostly meant as an implementation detail to be
    /// used by code generated by wayland-scanner.
    fn inert(backend: backend::WeakBackend) -> Self;

    /// Send a request for this object.
    ///
    /// It is an error to use this function on requests that create objects; use
    /// [Proxy::send_constructor] for such requests.
    fn send_request(&self, req: Self::Request) -> Result<(), InvalidId>;

    /// Send a request for this object that creates another object.
    ///
    /// It is an error to use this function on requests that do not create objects; use
    /// [Proxy::send_request] for such requests.
    fn send_constructor<I: Proxy>(
        &self,
        req: Self::Request,
        data: Arc<dyn ObjectData>,
    ) -> Result<I, InvalidId>;

    /// Parse a event for this object
    ///
    /// **Note:** This method is mostly meant as an implementation detail to be
    /// used by code generated by wayland-scanner.
    fn parse_event(
        conn: &Connection,
        msg: Message<ObjectId, OwnedFd>,
    ) -> Result<(Self, Self::Event), DispatchError>;

    /// Serialize a request for this object
    ///
    /// **Note:** This method is mostly meant as an implementation detail to be
    /// used by code generated by wayland-scanner.
    #[allow(clippy::type_complexity)]
    fn write_request(
        &self,
        conn: &Connection,
        req: Self::Request,
    ) -> Result<(Message<ObjectId, RawFd>, Option<(&'static Interface, u32)>), InvalidId>;

    /// Creates a weak handle to this object
    ///
    /// This weak handle will not keep the user-data associated with the object alive,
    /// and can be converted back to a full proxy using [`Weak::upgrade()`].
    ///
    /// This can be of use if you need to store proxies in the used data of other objects and want
    /// to be sure to avoid reference cycles that would cause memory leaks.
    fn downgrade(&self) -> Weak<Self> {
        Weak { backend: self.backend().clone(), id: self.id(), _iface: std::marker::PhantomData }
    }
}

/// Wayland dispatching error
#[derive(thiserror::Error, Debug)]
pub enum DispatchError {
    /// The received message does not match the specification for the object's interface.
    #[error("Bad message for object {interface}@{sender_id} on opcode {opcode}")]
    BadMessage {
        /// The id of the target object
        sender_id: ObjectId,
        /// The interface of the target object
        interface: &'static str,
        /// The opcode number
        opcode: u16,
    },
    /// The backend generated an error
    #[error("Backend error: {0}")]
    Backend(#[from] WaylandError),
}

/// A weak handle to a Wayland object
///
/// This handle does not keep the underlying user data alive, and can be converted back to a full proxy
/// using [`Weak::upgrade()`].
#[derive(Debug, Clone)]
pub struct Weak<I> {
    backend: WeakBackend,
    id: ObjectId,
    _iface: std::marker::PhantomData<I>,
}

impl<I: Proxy> Weak<I> {
    /// Try to upgrade with weak handle back into a full proxy.
    ///
    /// This will fail if either:
    /// - the object represented by this handle has already been destroyed at the protocol level
    /// - the Wayland connection has already been closed
    pub fn upgrade(&self) -> Result<I, InvalidId> {
        let backend = self.backend.upgrade().ok_or(InvalidId)?;
        let conn = Connection::from_backend(backend);
        I::from_id(&conn, self.id.clone())
    }

    /// The underlying [`ObjectId`]
    pub fn id(&self) -> ObjectId {
        self.id.clone()
    }
}

impl<I> PartialEq for Weak<I> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<I> Eq for Weak<I> {}

impl<I> Hash for Weak<I> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<I: Proxy> PartialEq<I> for Weak<I> {
    fn eq(&self, other: &I) -> bool {
        self.id == other.id()
    }
}
