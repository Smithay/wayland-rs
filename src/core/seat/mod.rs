//! Structures related to seats and inputs.
//!
//! A `Seat` represents a set of input devices, with possibly any combination
//! of a `Keyboard`, a `Pointer` and a `Touch` (not yet implemented). Most
//! classic settings will have a single `Seat`.
//!
//! These objects allow you to handle user input, using various callbacks.

pub use self::keyboard::{Keyboard, KeyboardId};
pub use self::pointer::{Pointer, PointerId};
pub use self::touch::{Touch, TouchId};
pub use self::seat::Seat;

pub use self::keyboard::KeymapFormat;
pub use self::keyboard::KeyState;
pub use self::pointer::ScrollAxis;
pub use self::pointer::ButtonState;

mod keyboard;
mod pointer;
mod seat;
mod touch;