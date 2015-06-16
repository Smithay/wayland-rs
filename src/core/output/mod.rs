//! Structures related to outputs.
//!
//! An `Output` represents a physical screen controlled by the Wayland server.
//! These objects are only meant to provide you information about the screens
//! of the computer your program is running on, for example to select the optimal
//! settings of a fullscreen application.

pub use self::output::{Output, OutputMode, OutputId};
pub use self::output::OutputTransform;
pub use self::output::OutputSubpixel;

mod output;