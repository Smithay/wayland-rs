//! Client-side Wayland connector
//!
//! ## Overview
//!
//! This crate provides the interfaces and machinery to safely create
//! client applications for the Wayland protocol. It can be used as a rust
//! implementation of the protocol or as a wrapper around the system-wide
//! `libwayland-client.so` if you enable the `use_system_lib` cargo feature.
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
//! objects, receiving and handling them is done by providing callbacks.
//!
//! ### Proxies
//!
//! Wayland objects are represented by proxies, which are handles to them.
//! You can interact with them in 4 states:
//!
//! - As the interface object directly `I`. This representation is the most immediate
//!   one. It allows you to send requests though this object and can be send accross threads.
//! - As a `Proxy<I>`. This representation is suitable if you want to access the proxy as
//!   a proxy, rather than a wayland object. You can convert between `I` and `Proxy<I>` via
//!   the `From` and `Into` traits, and get a `&Proxy<I>` from an `I` via the `AsRef` trait.
//! - As a `Main<I>`. This represents a main handle to this proxy, and allows you greater
//!   control of the object, but cannot be shared accros threads. This handle allows you to
//!   assign filters to the object, and send requests that create new objects.
//! - As an `Attached<I>`. If you use more than one event queue (see below), this allows you
//!   to control on which event queue the children object are created.
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
//!
//! Sending requests on dead objects will be silently ignored. And if these requests
//! would create new objects, these objects will be created dead.
//!
//! ### Filters
//!
//! Your wayland objects can receive events from the server, which need to be processed.
//! To do so, you can assign `Filter`s to your object. These are specially wrapped closure
//! so that several objects can be assigned to the same `Filter`, to ease state sharing
//! between the code handling different objects.
//!
//! If an object is not assigned to any `Filter`, its events will instead be delivered to the
//! fallback closure given to its event queue when dispatching it.
//!
//! ## Event Queues
//!
//! The Wayland client machinery provides the possibility to have one or more event queues
//! handling the processing of received messages. All Wayland objects are associated to an
//! event queue, which controls when its events are dispatched.
//!
//! Events received from the server are stored in an internal buffer, and processed (by calling
//! the appropriate callbacks) when the associated event queue is dispatched.
//!
//! When you send a request creating a new object, this new object will be assigned to an event
//! queue depending on the parent object that created it.
//!
//! - If the request was sent from a `Main<I>` handle, the child object will be assigned to the
//!   same event queue as its parent.
//! - If the request was sent from an `Attached<I>` handle, the child object will be assigned to
//!   the event queue its parent has been attached to.
//!
//! At the beginning you'll need to create an event queue and assign the initial `Proxy<WlDisplay>`
//! to it.
//!
//! ## Dynamic linking with `libwayland-client.so`
//!
//! If you need to gracefully handle the case of a system on which Wayland is not installed (by
//! fallbacking to X11 for example), you can do so by activating the `dlopen` cargo feature.
//!
//! When this is done, the library will be loaded a runtime rather than directly linked. And trying
//! to create a `Display` on a system that does not have this library will return a `NoWaylandLib`
//! error.

#![warn(missing_docs, missing_debug_implementations)]

#[macro_use]
extern crate bitflags;
#[cfg(not(feature = "use_system_lib"))]
#[macro_use]
extern crate downcast_rs as downcast;

#[cfg_attr(feature = "use_system_lib", macro_use)]
extern crate wayland_sys;

mod display;
mod event_queue;
mod globals;
mod proxy;

pub use anonymous_object::AnonymousObject;
pub use display::{ConnectError, Display, ProtocolError};
pub use event_queue::{EventQueue, QueueToken, ReadEventsGuard};
pub use globals::{GlobalError, GlobalEvent, GlobalImplementor, GlobalManager};
pub use imp::ProxyMap;
pub use proxy::{Attached, Main, Proxy};
pub use wayland_commons::{
    filter::{DispatchData, Filter},
    user_data::UserData,
    Interface, MessageGroup, NoMessage,
};

// rust implementation
#[cfg(not(feature = "use_system_lib"))]
#[path = "rust_imp/mod.rs"]
mod imp;
// C-lib based implementation
#[cfg(feature = "use_system_lib")]
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
    #![allow(missing_docs, clippy::all)]

    pub(crate) use crate::{AnonymousObject, Attached, Main, Proxy, ProxyMap};
    pub(crate) use wayland_commons::map::{Object, ObjectMetadata};
    pub(crate) use wayland_commons::smallvec;
    pub(crate) use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc};
    pub(crate) use wayland_commons::{Interface, MessageGroup};
    pub(crate) use wayland_sys as sys;
    include!(concat!(env!("OUT_DIR"), "/wayland_api.rs"));
}

mod anonymous_object {
    use super::{Interface, NoMessage, Proxy};
    use std::fmt::{self, Debug, Formatter};

    /// Anonymous interface
    ///
    /// A special Interface implementation representing an
    /// handle to an object for which the interface is not known.
    #[derive(Clone, Eq, PartialEq)]
    pub struct AnonymousObject(pub(crate) Proxy<AnonymousObject>);

    impl Interface for AnonymousObject {
        type Request = NoMessage;
        type Event = NoMessage;
        const NAME: &'static str = "<anonymous>";
        const VERSION: u32 = 0;
        fn c_interface() -> *const crate::sys::common::wl_interface {
            std::ptr::null()
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
    impl Debug for AnonymousObject {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_fmt(format_args!("{:?}", self.0))
        }
    }
}

/// Enum of possible argument in an event
#[derive(Debug)]
pub enum Argument {
    /// i32
    Int(i32),
    /// u32
    Uint(u32),
    /// float
    Float(f32),
    /// CString
    Str(Option<String>),
    /// id of a wayland object
    Object(Option<Proxy<AnonymousObject>>),
    /// id of a newly created wayland object
    NewId(Option<Main<AnonymousObject>>),
    /// Vec<u8>
    Array(Option<Vec<u8>>),
    /// RawFd
    Fd(std::os::unix::io::RawFd),
}

/// An generic event
#[derive(Debug)]
pub struct RawEvent {
    /// Interface of the associated object
    pub interface: &'static str,
    /// Opcode of the event
    pub opcode: u16,
    /// Name of the event
    pub name: &'static str,
    /// Arguments of the message
    pub args: Vec<Argument>,
}

/// Generate an enum joining several objects events
///
/// This macro allows you to easily create a enum type for use with your message Filters. It is
/// used like so:
///
/// ```no_run
/// # use wayland_client::protocol::{wl_pointer::WlPointer, wl_keyboard::WlKeyboard, wl_surface::WlSurface};
/// # use wayland_client::event_enum;
/// event_enum!(
///     MyEnum |
///     Pointer => WlPointer,
///     Keyboard => WlKeyboard,
///     Surface => WlSurface
/// );
/// ```
///
/// This will generate the following enum, unifying the events from each of the provided interface:
///
/// ```ignore
/// pub enum MyEnum {
///     Pointer { event: WlPointer::Event, object: Main<WlPointer> },
///     Keyboard { event: WlKeyboard::Event, object: Main<WlKeyboard> },
///     Surface { event: WlSurface::Event, object: Main<WlSurface> }
/// }
/// ```
///
/// It will also generate the appropriate `From<_>` implementation so that a `Filter<MyEnum>` can be
/// used as an implementation for `WlPointer`, `WlKeyboard` and `WlSurface`.
///
/// If you want to add custom messages to the enum, the macro also supports it:
///
/// ```no_run
/// # use wayland_client::protocol::{wl_pointer::WlPointer, wl_keyboard::WlKeyboard, wl_surface::WlSurface};
/// # use wayland_client::event_enum;
/// # struct SomeType;
/// # struct OtherType;
/// event_enum!(
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
///     Pointer { event: WlPointer::Event, object: Main<WlPointer> },
///     Keyboard { event: WlKeyboard::Event, object: Main<WlKeyboard> },
///     Surface { event: WlSurface::Event, object: Main<WlSurface> },
///     MyMessage(SomeType),
///     OtherMessage(OtherType)
/// }
/// ```
///
/// as well as implementations of `From<SomeType>` and `From<OtherType>`, so that these types can
/// directly be provided into a `Filter<MyEnum>`.
#[macro_export]
macro_rules! event_enum(
    ($(#[$attrs:meta])* $enu:ident | $($evt_name:ident => $iface:ty),*) => {
        $crate::event_enum!($(#[$attrs])* $enu | $($evt_name => $iface),* | );
    };
    ($(#[$attrs:meta])* $enu:ident | $($evt_name:ident => $iface:ty),* | $($name:ident => $value:ty),*) => {
        $(#[$attrs])*
        pub enum $enu {
            $(
                $evt_name { event: <$iface as $crate::Interface>::Event, object: $crate::Main<$iface> },
            )*
            $(
                $name($value),
            )*
        }

        $(
            impl From<($crate::Main<$iface>, <$iface as $crate::Interface>::Event)> for $enu {
                fn from((object, event): ($crate::Main<$iface>, <$iface as $crate::Interface>::Event)) -> $enu {
                    $enu::$evt_name { event, object }
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
