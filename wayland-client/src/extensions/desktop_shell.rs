//! desktop-shell protocol.
//!
//! To use it, you must activate the `wp-desktop_shell` cargo feature.

pub use sys::desktop_shell::client::{DesktopShell, Screensaver};
pub use sys::desktop_shell::client::DesktopProtocolEvent;
