#![cfg_attr(rustfmt, rustfmt_skip)]

//! Miscellaneous protocols
//!
//! Collections of protocols grabbed from here and there, but used
//! commonly enough to be worth adding in wayland-protocols.

pub mod server_decoration {
    //! KDE's server decoration protocol
    //!
    //! Allows you to negotiate with the compositor so that
    //! it decorates your windows, rather than having to do it yourself.
    wayland_protocol!(
        "server-decoration",
        [
            (wl_surface, wl_surface_interface)
        ]
    );
}
