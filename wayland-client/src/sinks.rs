//! Message sinks
//!
//! The Wayland model naturally uses callbacks to handle the message you receive from the server.
//! However in some contexts, an iterator-based interface may be more practical to use, this is
//! what this module provides.
//!
//! The `message_iterator` function allows you to create a new message iterator. It is just a
//! regular MPSC, however its sending end (the `Sink<T>`) can be directly used as an implementation
//! for Wayland objects. This just requires that `T: From<(I::Event, I)>` (where `I` is the
//! interface of the object you're trying to implement). The `event_enum!` macro is provided to
//! easily generate an appropriate type joining events from different interfaces into a single
//! iterator.
//!
//! The `blocking_message_iterator` function is very similar, except the created message iterator
//! will be linked to an event queue, and will block on it rather than returning `None`, and is
//! thus able to drive an event loop.

use std::rc::Rc;

use wayland_commons::Interface;

use imp::EventQueueInner;
use {HandledBy, QueueToken};

pub use wayland_commons::sinks::{message_iterator, MsgIter, Sink};

impl<M, I> HandledBy<Sink<M>> for I
where
    I: Interface,
    M: From<(I::Event, I)>,
{
    fn handle(sink: &mut Sink<M>, event: I::Event, proxy: I) {
        sink.push((event, proxy));
    }
}

/// A message iterator linked to an event queue
///
/// Like a `MsgIter<T>`, but it is linked with an event queue, and
/// will `dispatch()` and block instead of returning `None` if no
/// events are pending.
pub struct BlockingMsgIter<T> {
    evt_queue: Rc<EventQueueInner>,
    iter: MsgIter<T>,
}

impl<T> Iterator for BlockingMsgIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        loop {
            match self.iter.next() {
                Some(msg) => return Some(msg),
                None => {
                    self.evt_queue
                        .dispatch()
                        .expect("Connection to the wayland server lost.");
                }
            }
        }
    }
}

/// Create a blokcing message iterator
///
/// Contrarily to `message_iterator`, this one will block on the event queue
/// represented by the provided token rather than returning `None` if no event is available.
pub fn blocking_message_iterator<T>(token: QueueToken) -> (Sink<T>, BlockingMsgIter<T>) {
    let (sink, iter) = message_iterator();
    (
        sink,
        BlockingMsgIter {
            iter,
            evt_queue: token.inner,
        },
    )
}

/// Generate an enum joining several objects events
///
/// This macro allows you to easily create a enum type for use with your message iterators. It is
/// used like so:
///
/// ```ignore
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
///     Pointer { event: WlPointer::Event, object: WlPointer },
///     Keyboard { event: WlKeyboard::Event, object: WlKeyboard },
///     Surface { event: WlSurface::Event, object: WlSurface }
/// }
/// ```
///
/// It will also generate the appropriate `From<_>` implementation so that a `Sink<MyEnum>` can be
/// used as an implementation for `WlPointer`, `WlKeyboard` and `WlSurface`.
///
/// If you want to add custom messages to the enum, the macro also supports it:
///
/// ```ignore
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
///     Pointer { event: WlPointer::Event, object: WlPointer },
///     Keyboard { event: WlKeyboard::Event, object: WlKeyboard },
///     Surface { event: WlSurface::Event, object: WlSurface },
///     MyMessage(SomeType),
///     OtherMessage(OtherType)
/// }
/// ```
///
/// as well as implementations of `From<SomeType>` and `From<OtherType>`, so that these types can
/// directly be provided into a `Sink<MyEnum>`.

#[macro_export]
macro_rules! event_enum(
    ($enu:ident | $($evt_name:ident => $iface:ty),*) => {
        event_enum!($enu | $($evt_name => $iface),* | );
    };
    ($enu:ident | $($evt_name:ident => $iface:ty),* | $($name:ident => $value:ty),*) => {
        pub enum $enu {
            $(
                $evt_name { event: <$iface as $crate::Interface>::Event, object: $iface },
            )*
            $(
                $name($value)
            )*
        }

        $(
            impl From<(<$iface as $crate::Interface>::Event, $iface)> for $enu {
                fn from((event, object): (<$iface as $crate::Interface>::Event, $iface)) -> $enu {
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
