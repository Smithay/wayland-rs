//! [unstable] XDG Shell protocol.
//!
//! To use it, you must activate the `wpu-xdg_shell` cargo feature.
//!
//! This is an unstable protocol. To use it, you must set the `unstable-protocols` cargo feature
//! and use a nightly protocol.
//!
//! This protocol is currently under heavy developpment and destined to eventually replace
//! the `wl_shell` interface of the core protocol.

pub use sys::xdg_shell::server::{XdgShell, XdgShellRequest};
pub use sys::xdg_shell::server::{XdgSurface, XdgSurfaceResizeEdge, XdgSurfaceState, XdgSurfaceRequest};
pub use sys::xdg_shell::server::{XdgPopup, XdgPopupRequest};

pub use sys::xdg_shell::server::XdgShellUnstableV5ProtocolRequest;

protocol_globals!(XdgShellUnstableV5, XdgShellUnstableV5ProtocolGlobalInstance,
    Shell => ::xdg_shell::XdgShell
);
