//! [unstable] XDG Shell protocol.
//!
//! To use it, you must activate the `wpu-xdg_shell` cargo feature.
//!
//! This is an unstable protocol. To use it, you must set the `unstable-protocols` cargo feature
//! and use a nightly protocol.
//!
//! This protocol is currently under heavy developpment and destined to eventually replace
//! the `wl_shell` interface of the core protocol.

pub use sys::xdg_shell::client::{XdgShell, XdgShellEvent};
pub use sys::xdg_shell::client::{XdgSurface, XdgSurfaceResizeEdge, XdgSurfaceState, XdgSurfaceEvent};
pub use sys::xdg_shell::client::{XdgPopup, XdgPopupEvent};

pub use sys::xdg_shell::client::XdgShellUnstableV5ProtocolEvent;