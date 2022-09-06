//! Interface for interacting with the Wayland protocol, server-side.
//!
//! ## General concepts
//!
//! This crate is structured around four main objects: the [`Display`] and [`DisplayHandle`] structs,
//! resources (objects implementing the [`Resource`] trait), and the [`Dispatch`] trait.
//!
//! The [`Display`] is the heart of this crate, it represents the protocol state of your Wayland server, and
//! takes care of processing messages from clients. You'll need to integrate it in your event loop (see its
//! documentation for details). From it you can retrieve the [`DisplayHandle`], which is a clonable handle to
//! the Wayland state and is the type used to actually interact with the protocol.
//!
//! Each of the Wayland object you can manipulate is represented by a struct implementing the [`Resource`]
//! trait. Thos structs are automatically generated from the wayland XML protocol specification. This crate
//! provides the types generated from the core protocol in the [`protocol`] module. For other standard
//! protocols, see the `wayland-protocols` crate.
//!
//! ## Request dispatching and the [`Dispatch`] trait
//!
//! The request dispatching logic provided by this crate is build around the [`Dispatch`] trait. During the
//! dispatching process (in [`Display::dispatch_clients()`]), all requests sent by clients are read from
//! their respective process and delivered to your processing logic, by invoking methods on the various
//! [`Dispatch`] implementations of your `State` struct. In this paradigm, your `State` needs to implement
//! `Dispatch<O, _>` for every Wayland object `O` it needs to process events for.
//!
//! However, implementing all those traits on your own is a lot of (often uninteresting) work. To make this
//! easier a composition mechanism is provided using the [`delegate_dispatch!`] macro. This way, another
//! library (such as Smithay) can provide generic [`Dispatch`] implementations that you can reuse on your
//! own app by delegating those objects to that provided implementation. See the documentation of those
//! traits and macro for details.
//!
//! ## Globals
//!
//! The entry point of the protocol for clients goes through the protocol globals. Each global represents a
//! capability of your compositor, a peripheral it has access to, or a protocol extension it supports.
//! Globals are created by you using [`DisplayHandle::create_global()`], and require your `State` to
//! implement the [`GlobalDispatch`] trait for the interface associated with that global.
//!
//! ## Logging
//!
//! This crate can generate some runtime error message (notably when a protocol error occurs). By default
//! those messages are printed to stderr. If you activate the `log` cargo feature, they will instead be
//! piped through the `log` crate.
//!
//! ## Advanced use
//!
//! ### Bypassing [`Dispatch`]
//!
//! It may be that for some of your objects, handling them via the [`Dispatch`] trait is impractical. In
//! those contexts, this crate also provides some escape-hatches to directly interface with the low-level
//! APIs from `wayland-backend`, allowing you to register callbacks for those objects by directly providing
//! implementations of the backend [`ObjectData`](crate::backend::ObjectData) trait.
//! See [`Client::create_resource_from_objdata()`] and [`DataInit::custom_init()`].
//!
//! ### Interaction with FFI
//!
//! It can happen that you'll need to interact with Wayland states accross FFI, such as for example when
//! interfacing with the graphics stack for enabling hardware acceleration for clients.
//!
//! In this case, you'll need to do it in two steps, by explicitly working with `wayland-backend`, adding
//! it to your dependencies and enabling its `server_system` feature.
//!
//! Then, you'll generally need:
//!
//! - The `*mut wl_display` pointer, that you can retrieve by first retrieving the
//!   [`Backend`](crate::backend::Backend) using [`Display::backend()`], and then invoke
//!   `Backend::display_ptr().
//! - The `*mut wl_resource` pointers for the objects you need to share, by first getting the
//!   [`ObjectId`](crate::backend::ObjectId) using the [`Resource::id()`] method, and then
//!   the `ObjectId::as_ptr()` method.
//!
//! If you need to receive pointers from FFI, you can make [`ObjectId`]s from the `*mut wl_resource` pointers
//! using `ObjectId::from_ptr()`, and then make the resources using [`Resource::from_id`].
#![forbid(improper_ctypes, unsafe_op_in_unsafe_fn)]

use wayland_backend::{
    protocol::{Interface, Message},
    server::{ClientId, InvalidId, ObjectId, WeakHandle},
};

mod client;
mod dispatch;
mod display;
mod global;
mod socket;

pub use client::Client;
pub use dispatch::{DataInit, Dispatch, New, ResourceData};
pub use display::{Display, DisplayHandle};
pub use global::GlobalDispatch;
pub use socket::{BindError, ListeningSocket};

/// Backend reexports
pub mod backend {
    pub use wayland_backend::protocol;
    pub use wayland_backend::server::{
        Backend, ClientData, ClientId, Credentials, DisconnectReason, GlobalHandler, GlobalId,
        Handle, InitError, InvalidId, ObjectData, ObjectId, WeakHandle,
    };
    pub use wayland_backend::smallvec;
}

pub use wayland_backend::protocol::WEnum;

/// Generated protocol definitions
///
/// This module is automatically generated from the `wayland.xml` protocol specification, and contains the
/// interface definitions for the core Wayland protocol.
#[allow(missing_docs)]
pub mod protocol {
    use self::__interfaces::*;
    use crate as wayland_server;
    pub mod __interfaces {
        wayland_scanner::generate_interfaces!("wayland.xml");
    }
    wayland_scanner::generate_server_code!("wayland.xml");
}

// internal imports for dispatching logging depending on the `log` feature
#[cfg(feature = "log")]
#[allow(unused_imports)]
use log::{debug as log_debug, error as log_error, info as log_info, warn as log_warn};
#[cfg(not(feature = "log"))]
#[allow(unused_imports)]
use std::{
    eprintln as log_error, eprintln as log_warn, eprintln as log_info, eprintln as log_debug,
};

/// Trait representing a Wayland interface
pub trait Resource: Clone + std::fmt::Debug + Sized {
    /// The event enum for this interface
    type Event;
    /// The request enum for this interface
    type Request;

    /// The interface description
    fn interface() -> &'static Interface;

    /// The ID of this object
    fn id(&self) -> ObjectId;

    /// The ID of the client owning this object
    ///
    /// Returns [`None`] if the object is no longer alive.
    fn client_id(&self) -> Option<ClientId> {
        self.handle().upgrade().and_then(|dh| dh.get_client(self.id()).ok())
    }

    /// The version of this object
    fn version(&self) -> u32;

    /// Checks if the Wayland object associated with this proxy is still alive
    fn is_alive(&self) -> bool {
        if let Some(handle) = self.handle().upgrade() {
            handle.object_info(self.id()).is_ok()
        } else {
            false
        }
    }

    /// Access the user-data associated with this object
    fn data<U: 'static>(&self) -> Option<&U>;

    /// Access the raw data associated with this object.
    ///
    /// It is given to you as a `dyn Any`, and you are responsible for downcasting it.
    ///
    /// For objects created using the scanner-generated methods, this will be an instance of the
    /// [`ResourceData`] type.
    fn object_data(&self) -> Option<&std::sync::Arc<dyn std::any::Any + Send + Sync>>;

    /// Access the backend handle associated with this object
    fn handle(&self) -> &backend::WeakHandle;

    /// Create an object resource from its ID
    ///
    /// Returns an error this the provided object ID does not correspond to the `Self` interface.
    ///
    /// **Note:** This method is mostly meant as an implementation detail to be used by code generated by
    /// wayland-scanner.
    fn from_id(dh: &DisplayHandle, id: ObjectId) -> Result<Self, InvalidId>;

    /// Send an event to this object
    fn send_event(&self, evt: Self::Event) -> Result<(), InvalidId>;

    /// Trigger a protocol error on this object
    ///
    /// The `code` is intended to be from the `Error` enum declared alongside that object interface.
    ///
    /// A protocol error is fatal to the Wayland connection, and the client will be disconnected.
    #[inline]
    fn post_error(&self, code: impl Into<u32>, error: impl Into<String>) {
        if let Some(dh) = self.handle().upgrade().map(DisplayHandle::from) {
            dh.post_error(self, code.into(), error.into());
        }
    }

    /// Parse a event for this object
    ///
    /// **Note:** This method is mostly meant as an implementation detail to be used by code generated by
    /// wayland-scanner.
    fn parse_request(
        dh: &DisplayHandle,
        msg: Message<ObjectId>,
    ) -> Result<(Self, Self::Request), DispatchError>;

    /// Serialize an event for this object
    ///
    /// **Note:** This method is mostly meant as an implementation detail to be used by code generated by
    /// wayland-scanner.
    fn write_event(
        &self,
        dh: &DisplayHandle,
        req: Self::Event,
    ) -> Result<Message<ObjectId>, InvalidId>;

    /// Creates a weak handle to this object
    ///
    /// This weak handle will not keep the user-data associated with the object alive,
    /// and can be converted back to a full resource using [`Weak::upgrade()`].
    ///
    /// This can be of use if you need to store resources in the used data of other objects and want
    /// to be sure to avoid reference cycles that would cause memory leaks.
    fn downgrade(&self) -> Weak<Self> {
        Weak { handle: self.handle().clone(), id: self.id(), _iface: std::marker::PhantomData }
    }

    #[doc(hidden)]
    fn __set_object_data(
        &mut self,
        odata: std::sync::Arc<dyn std::any::Any + Send + Sync + 'static>,
    );
}

/// An error generated if an illegal request was received from a client
#[derive(thiserror::Error, Debug)]
pub enum DispatchError {
    /// The received message does not match the specification for the object's interface.
    #[error("Bad message for interface {interface} : {msg:?}")]
    BadMessage {
        /// The received message
        msg: Message<ObjectId>,
        /// The interface of the object that received it
        interface: &'static str,
    },
}

/// A weak handle to a Wayland object
///
/// This handle does not keep the underlying user data alive, and can be converted back to a full resource
/// using [`Weak::upgrade()`].
#[derive(Debug, Clone)]
pub struct Weak<I> {
    handle: WeakHandle,
    id: ObjectId,
    _iface: std::marker::PhantomData<I>,
}

impl<I: Resource> Weak<I> {
    /// Try to upgrade with weak handle back into a full resource.
    ///
    /// This will fail if either:
    /// - the object represented by this handle has already been destroyed at the protocol level
    /// - the Wayland connection has already been closed
    pub fn upgrade(&self) -> Result<I, InvalidId> {
        let handle = self.handle.upgrade().ok_or(InvalidId)?;
        let d_handle = DisplayHandle::from(handle);
        I::from_id(&d_handle, self.id.clone())
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

impl<I: Resource> PartialEq<I> for Weak<I> {
    fn eq(&self, other: &I) -> bool {
        self.id == other.id()
    }
}
