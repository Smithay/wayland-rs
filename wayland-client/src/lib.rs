//! Client-side Wayland connector
//!
//! ## Overview
//!
//! This crate provides the interfaces and machinery to safely create
//! client applications for the Wayland protocol. It is a rust wrapper
//! around the `libwayland-client.so` C library.
//!
//! The Wayland protocol revolves around the creation of various objects
//! and the exchange of messages associated to these objects. The initial
//! object is always the `Display`, that you get at initialization of the
//! connection, exposed by this crate as `Display::connect_to_env()`.
//!
//! ## Protocol and messages handling model
//!
//! The protocol being bi-directional, you can send and receive messages.
//! Sending messages is done via methods of Rust objects corresponding to the wayland protocol
//! objects, receiving and handling them is done by providing implementations.
//!
//! ### Proxies
//!
//! The underlying representation of Wayland protocol objects in this crate is `Proxy<I>`,
//! where `I` is the type of the considered Rust object. An object's interface (think "class"
//! in an object-oriented context) defines which messages it can send and receive.
//!
//! These proxies are used to send messages to the server (in the Wayland context,
//! these are called "requests"). You usually don't use them directly, and instead call
//! methods on the Rust objects themselves, which invoke the appropriate `Proxy` methods.
//! It is also possible to directly use the `Proxy::<I>::send(..)` method, but
//! this should only be done carefully: using it improperly can mess the protocol
//! state and cause protocol errors, which are fatal to the connection (the server
//! will kill you).
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
//! and the `alive()` method on the underlying `Proxy<I>` will start to return `false`.
//! Trying to send messages with them will also fail.
//!
//! ### Implementations
//!
//! To receive and process messages from the server to you (in Wayland context they are
//! called "events"), you need to provide an `Implementation` for each Wayland object
//! created in the protocol session. Whenever a new protocol object is created, you will
//! receive a `NewProxy<I>` object. Providing an implementation via its `implement()` method
//! will turn it into a regular Rust object.
//!
//! **All objects must be implemented**, even if it is an implementation doing nothing.
//! Failure to do so (by dropping the `NewProxy<I>` for example) can cause future fatal
//! errors if the server tries to send an event to this object.
//!
//! An implementation is a struct implementing the `EventHandler` trait for the interface
//! of the considered object. Alternatively, an `FnMut(I::Event, I)` closure can be
//! used with the `implement_closure()` method, where `I` is the interface
//! of the considered object.
//!
//! ## Event Queues
//!
//! The Wayland client machinery provides the possibility to have one or more event queues
//! handling the processing of received messages. All Wayland objects are associated to an
//! event queue, which controls when its events are dispatched.
//!
//! Events received from the server are stored in an internal buffer, and processed (by calling
//! the appropriate implementations) when the associated event queue is dispatched.
//!
//! A default event queue is created at the same time as the initial `Display`, and by default
//! whenever a Wayland object is created, it inherits the queue of its parent (the object that sent
//! or receive the message that created the new object). It means that if you only plan to use the
//! default event queue, you don't need to worry about assigning objects to their queues.
//!
//! See the documentation of `EventQueue` for details about dispatching and integrating the event
//! queue into the event loop of your application. See the `Proxy::make_wrapper()` method for
//! details about assigning objects to event queues.
//!
//! ## Dynamic linking with `libwayland-client.so`
//!
//! If you need to gracefully handle the case of a system on which Wayland is not installed (by
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
//!   OpenGL surface from Wayland surfaces.
//!
//! Both of them will also be loaded at runtime if the `dlopen` feature was provided. See their
//! respective submodules for details about their use.
//!
//! ### Event Loop integration
//!
//! The `eventloop` cargo feature adds the necessary implementations to use an `EventQueue`
//! as a `calloop` event source. If you want to use it, here are a few points to take into
//! account:
//!
//! - The `EventQueue` will not call its associated callback, but rather manage all the
//!   event dispatching internally. As a result, there is no point registering it to
//!   `calloop` with anything other than a dummy callback.
//! - You still need to call `Display::flush()` yourself between `calloop`s dispatches,
//!   or in the `EventLoop::run()` callback of `calloop`.

#![warn(missing_docs)]

#[macro_use]
extern crate bitflags;
#[cfg(not(feature = "native_lib"))]
#[macro_use]
extern crate downcast_rs as downcast;
extern crate libc;
extern crate nix;

#[cfg(feature = "eventloop")]
extern crate calloop;
#[cfg(feature = "eventloop")]
extern crate mio;

extern crate wayland_commons;
#[cfg_attr(feature = "native_lib", macro_use)]
extern crate wayland_sys;

mod display;
mod event_queue;
mod globals;
mod proxy;

pub use display::{ConnectError, Display, ProtocolError};
pub use event_queue::{EventQueue, QueueToken, ReadEventsGuard};
pub use globals::{GlobalError, GlobalEvent, GlobalImplementor, GlobalManager};
pub use imp::ProxyMap;
pub use proxy::{HandledBy, NewProxy, Proxy};

#[cfg(feature = "cursor")]
pub mod cursor;

#[cfg(feature = "egl")]
pub mod egl;

pub mod sinks;

pub use anonymous_object::AnonymousObject;
pub use wayland_commons::{Interface, MessageGroup, NoMessage};

// rust implementation
#[cfg(not(feature = "native_lib"))]
#[path = "rust_imp/mod.rs"]
mod imp;
// C-lib based implementation
#[cfg(feature = "native_lib")]
#[path = "native_lib/mod.rs"]
mod imp;

/// C-associated types
///
/// Required for plugging wayland-scanner generated protocols
/// or interfacing with C code using wayland objects.
pub mod sys {
    pub use wayland_sys::{client, common};
}

pub mod protocol {
    #![allow(dead_code, non_camel_case_types, unused_unsafe, unused_variables)]
    #![allow(non_upper_case_globals, non_snake_case, unused_imports)]
    #![allow(missing_docs)]
    #![cfg_attr(feature = "cargo-clippy", allow(clippy))]

    pub(crate) use wayland_commons::map::{Object, ObjectMetadata};
    pub(crate) use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc};
    pub(crate) use wayland_commons::{Interface, MessageGroup};
    pub(crate) use wayland_sys as sys;
    pub(crate) use {AnonymousObject, HandledBy, NewProxy, Proxy, ProxyMap};
    include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
}

mod anonymous_object {
    use super::{Interface, NoMessage, Proxy};

    /// Anonymous interface
    ///
    /// A special Interface implementation representing an
    /// handle to an object for which the interface is not known.
    #[derive(Clone, Eq, PartialEq)]
    pub struct AnonymousObject(Proxy<AnonymousObject>);

    impl Interface for AnonymousObject {
        type Request = NoMessage;
        type Event = NoMessage;
        const NAME: &'static str = "<anonymous>";
        const VERSION: u32 = 0;
        fn c_interface() -> *const ::sys::common::wl_interface {
            ::std::ptr::null()
        }
    }

    impl AsRef<Proxy<AnonymousObject>> for AnonymousObject {
        #[inline]
        fn as_ref(&self) -> &Proxy<Self> {
            &self.0
        }
    }
    impl From<Proxy<AnonymousObject>> for AnonymousObject {
        #[inline]
        fn from(proxy: Proxy<Self>) -> Self {
            AnonymousObject(proxy)
        }
    }
    impl From<AnonymousObject> for Proxy<AnonymousObject> {
        #[inline]
        fn from(value: AnonymousObject) -> Self {
            value.0
        }
    }
}

#[cfg(feature = "eventloop")]
impl ::mio::event::Evented for EventQueue {
    fn register(
        &self,
        poll: &::mio::Poll,
        token: ::mio::Token,
        interest: ::mio::Ready,
        opts: ::mio::PollOpt,
    ) -> ::std::io::Result<()> {
        let fd = self.inner.get_connection_fd();
        ::mio::unix::EventedFd(&fd).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &::mio::Poll,
        token: ::mio::Token,
        interest: ::mio::Ready,
        opts: ::mio::PollOpt,
    ) -> ::std::io::Result<()> {
        let fd = self.inner.get_connection_fd();
        ::mio::unix::EventedFd(&fd).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &::mio::Poll) -> ::std::io::Result<()> {
        let fd = self.inner.get_connection_fd();
        ::mio::unix::EventedFd(&fd).deregister(poll)
    }
}

#[cfg(feature = "eventloop")]
impl ::calloop::EventSource for EventQueue {
    type Event = ();

    fn interest(&self) -> ::mio::Ready {
        ::mio::Ready::readable()
    }

    fn pollopts(&self) -> ::mio::PollOpt {
        ::mio::PollOpt::edge()
    }

    fn make_dispatcher<Data: 'static, F: FnMut((), &mut Data) + 'static>(
        &self,
        _callback: F,
    ) -> ::std::rc::Rc<::std::cell::RefCell<::calloop::EventDispatcher<Data>>> {
        struct Dispatcher {
            inner: ::std::rc::Rc<::imp::EventQueueInner>,
        }

        impl<Data> ::calloop::EventDispatcher<Data> for Dispatcher {
            fn ready(&mut self, _ready: ::mio::Ready, _data: &mut Data) {
                if let Err(()) = self.inner.prepare_read() {
                    self.inner.dispatch_pending().unwrap();
                } else {
                    match self.inner.read_events() {
                        Ok(_) => {
                            self.inner.dispatch_pending().unwrap();
                        }
                        Err(e) => match e.kind() {
                            ::std::io::ErrorKind::WouldBlock => {}
                            _ => {
                                panic!("Failed to read from wayland socket: {}", e);
                            }
                        },
                    }
                }
            }
        }

        ::std::rc::Rc::new(::std::cell::RefCell::new(Dispatcher {
            inner: self.inner.clone(),
        }))
    }
}
