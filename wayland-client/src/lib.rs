#![cfg_attr(feature = "unstable-protocols", feature(static_recursion))]
#![deny(missing_docs)]

//! Wayland client library bindings
//!
//! This crate offers relatively low-level bindings to the client-side
//! wayland API. Allowing you to write wayland applications mostly without
//! the need of unsafe code.
//!
//! ## The Wayland protocol
//!
//! The protocol used between clients and compositors in the wayland architecture is
//! composed of objects. Each object is the instanciation of an interface, which
//! defines several requests and events.
//!
//! Requests are messages from the client to the compositor, events are messages from
//! the compositor to the client. Both are always associated with an object.
//!
//! These objects are represented in this crate via the `Proxy` trait, and can be
//! identified via the `ProxyId` tokens, which can be compared for equality, or used
//! as a key in an hashmap.
//!
//! Requests of an object can be invoked as methods on the proxy object representing
//! it. Events are retrieved fia events iterator, which yields the event, associated
//! to a `ProxyId`, to identify which object they are related to.
//!
//! See the documentation of the `wayland` module for details about the main interfaces
//! of the core protocol.
//!
//! ## Using this crate
//!
//! The entry point of this crate is the `get_display` function. It'll try to
//! open a connexion to the wayland compositor as defined by the environment
//! variables. On success, it'll provide you with the proxy associated to the
//! `wl_display`, as well as an event iterator, collecting its events.
//!
//! The first thing you'll do is retrieve the global objects advertized by the
//! compositor. You can to that by creating a `wl_registry` from your display,
//! and manually handling its events, or use the `wayland_env!()` macro which
//! abstracts away most of this job for you.
//!
//! Once this done you can start to create objects, setup your UI, and then
//! write you event loops.
//!
//! An example of squeletton using this structure is:
//!
//! ```no_run
//! #[macro_use] extern crate wayland_client;
//! use wayland_client::wayland::get_display;
//!
//! wayland_env!(WaylandEnv,
//!     /* list of the globals you want to use */
//! );
//! 
//!
//! fn main() {
//!     let (display, event_iter) = get_display()
//!         .expect("Unable to connect to the wayland compositor.");
//!     let (env, mut event_iter) = WaylandEnv::init(display, event_iter);
//!
//!     /* Setup your wayland objects */
//!
//!     loop {
//!         // Event loop
//!         for event in &mut event_iter {
//!             /* Handle the event */
//!         }
//!         // Sync pending messages with the compositor
//!         event_iter.dispatch().expect("Connexion with the wayland compositor was lost.");
//!     }
//! }
//! ```
//!
//! ## Modularity
//!
//! Many of the features offered by this crate are optionnal, and controlled by cargo features.
//!
//! ### Linking behavior
//!
//! By default, the crate is dynamically linked with the several C library it requires
//! (at least `libwayland-client.so` and maybe others, depending on your enabled features).
//! This can be changed with the feature `dlopen`. When activated, it'll cause the library
//! to instead try to load the libraries at runtime, and return an error if it failed,
//! allowing you to gracefully handle the lack of wayland support on the system, for example
//! by falling-back to X11.
//!
//! ### Secondary libraries
//!
//! This crate allows you to use two secondary C library provided by the official
//! wayland distribution:
//!
//! - `libwayland-egl.so`: this library is necessary is you plan to use OpenGL, it
//!   provides the glue necessary to intialize an OpenGL context. it i enabled by
//!   the `egl` cargo feature, and adds an `egl` submodule to the crate.
//! - `libwayland-cursor.so`: this library allows you to retrieve the cursor themes
//!   of the system your application is running on, in order to use them and integrate
//!   in the look and feel of the compositor ruuning your program.
//!
//! ### Protocol extensions
//!
//! Other cargo features include the support of various protocol extensions of wayland.
//! Such extentions add new objects to the supported ones, and are all optionnal.
//!
//! Each of them is controlled by a cargo feature, following this naming convention:
//!
//! - `wp_<feature name>` for protocol extensions whose specification are stable and
//!   can be relied upon
//! - `wpu_<feature name>` for protocol extensions whose specification are still under
//!   developpement and subject to drastic changes. No stability guarantee is provided
//!   by this crate for part of the API behiond these features.
//!
//! Each of these feature creates a submodule in the `extensions` module containing
//! the API of this extension. The documentation hosted online is generated with all
//! supported features activated.
//!
//! To use `wpu_*` protocol extensions, you'll also need to activate the `unstable-protocols`
//! cargo feature, which serves as a global gate for them. Some of them may also
//! require using a nightly rust compiler.
//!
//! ### Protocol errors
//!
//! This library is a safe wrapper around the C library in the rust sense. It'll gaurd
//! you from memory safety or data-races errors, but not from logic errors.
//!
//! Logic errors in wayland are called *Protocol Errors*. The documentation states
//! when doing something is a protocol error. Most basic example would be assigning
//! two different roles to a surface at the same time (see `wayland` module
//! documentation for details about what surfaces and roles are).
//!
//! When a protocol error is raised, the compositor will simply report it an close
//! the connexion to your client. When this happen, message-syncing methods of
//! `EventIterator` will start returning `Err(_)`.
//!
//! There is no way to recover from a protocol error, apart creating a brand new
//! wayland connexion and starting all over. Thus the `Iterator` implementation
//! of `EventIterator` will panic if a protocol error is raised. If you need to
//! gracefully handle protocol errors, use the manual envent querying methods instead.
//!
//! There is currently no way to query what the error was, but it is planned. See
//! [the associated github issue](https://github.com/vberger/wayland-client-rs/issues/33).

#[macro_use] extern crate bitflags;
extern crate libc;
#[macro_use] extern crate wayland_sys;

use std::sync::Arc;

mod env;
mod events;
mod sys;

#[cfg(feature = "egl")]
pub mod egl;

#[cfg(feature = "cursor")]
pub mod cursor;

pub mod wayland;

pub mod extensions;

use wayland_sys::client::wl_proxy;
use wayland_sys::common::wl_interface;

pub use wayland::{get_display, ConnectError};

pub use events::{Event, EventIterator, ReadEventsGuard};
use events::EventFifo;

/// Common routines for manipulating wayland objects
///
/// A proxy is the wayland term to represent an handle to a wayland object.
/// This trait defines a set of utility methods for these objects, that you'll be
/// able to use to identify them and handle their events.
pub trait Proxy : ProxyInternal {
    /// Get a pointer to the internal wayland object wrapped by this struct
    ///
    /// You'll only need this for interoperation with C libraries needing it, to setup
    /// an OpenGL or Vulkan context, for example.
    fn ptr(&self) -> *mut wl_proxy;
    /// Get a pointer to the definition of the interface implemented by this object.
    ///
    /// You'll only need this for interoperation with C libraries needing it.
    fn interface() -> *mut wl_interface;
    /// The internal name of this interface, as advertized by the registry if it is a global.
    fn interface_name() -> &'static str;
    /// The maximum version of this interface handled by the library.
    fn version() -> u32;
    /// Get the id of this proxy
    fn id(&self) -> ProxyId;
    /// Set the event iterator associated to this wayland object
    ///
    /// Once set, all events generated by this object will be dispatched to this EventIterator,
    /// an no other.
    ///
    /// By default an object inherits the event iterator of the object that created it.
    fn set_event_iterator(&mut self, iter: &EventIterator);
}

/// Trait used internally for implementation details.
#[doc(hidden)]
pub trait ProxyInternal {
    /// Creates a proxy from a fresh ptr
    unsafe fn from_ptr(ptr: *mut wl_proxy) -> Self;
    /// Creates a proxy from a ptr that is managed elsewhere
    ///
    /// As opposed to `from_ptr`, this function will not try to
    /// set a listener/dispatcher for this proxy, and thus its
    /// events won't be available.
    ///
    /// The created object _should_ be leaked, or it will destroy
    /// the ressource on drop, which will most likely trigger
    /// protocol errors.
    unsafe fn from_ptr_no_own(ptr: *mut wl_proxy) -> Self;
    /// Sets the event queue manually
    unsafe fn set_evq(&mut self, internals: Arc<EventFifo>);
}

/// An opaque identifier for Proxys
///
/// This opaque struct can be cloned and teste for equality. It serves
/// to identify which proxy an given event was coming from, in case you
/// have several events of the same type.
#[derive(Copy,Clone,PartialEq,Eq,Debug,Hash)]
pub struct ProxyId { id: usize }

fn wrap_proxy(ptr: *mut wl_proxy) -> ProxyId {
    ProxyId { id: ptr as usize}
}

/// Checks if the wayland lib is available
///
/// If the `dlopen` feature is disabled, will always return `true`.
/// If it is enabled, will return `true` if the wayland-client lib
/// is available and can be used.
pub fn is_available() -> bool {
    ::wayland_sys::client::is_lib_available()
}
