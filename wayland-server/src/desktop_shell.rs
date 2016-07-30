//! desktop-shell protocol.
//!
//! To use it, you must activate the `wl-desktop_shell` cargo feature.

pub use sys::desktop_shell::server::{DesktopShell, Screensaver};
pub use sys::desktop_shell::server::DesktopProtocolRequest;
