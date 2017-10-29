//! Protocols from the wayland-wall repository
//!
//! These are a collection of protocol extensions that is not
//! hosted on the official wayland-protocols repository, because
//! they do not align with the needs of the major DEs.
//!
//! They are hosted at https://github.com/wayland-wall/wayland-wall
//!
//! This module is behind the `wall-protocols` cargo feature.

#![cfg_attr(rustfmt, rustfmt_skip)]

/// This module hosts the unstable wayland-wall protocols
///
/// They are similarly versionned as the unstable protocols
/// from wayland-protocols.
///
/// This module is available only with both the `wall-protocols`
/// and `unstable-protocols` cargo features enabled.
#[cfg(feature = "unstable_protocols")]
pub mod unstable {
    pub mod background {
        //! This protocol will allow clients to display a surface as your screen background
        wayland_protocol_versioned!(
            "background",
            [v1, v2],
            [
                (wl_output, wl_output_interface),
                (wl_surface, wl_surface_interface)
            ]
        );
    }

    pub mod dock_manager {
        //! This protocol will allow clients to dock surfaces on screen edges
        wayland_protocol_versioned!(
            "dock-manager",
            [v1, v2],
            [
                (wl_output, wl_output_interface),
                (wl_surface, wl_surface_interface)
            ]
        );
    }

    pub mod launcher_menu {
        //! This protocol will allow launcher and menu clients, with a proper implicit grab
        wayland_protocol_versioned!(
            "launcher-menu",
            [v1],
            [
                (wl_seat, wl_seat_interface),
                (wl_surface, wl_surface_interface)
            ]
        );
    }

    pub mod notification_area {
        //! This protocol will allow notification daemons to display their notifications using Wayland
        wayland_protocol_versioned!(
            "notification-area",
            [v1],
            [
                (wl_surface, wl_surface_interface)
            ]
        );
    }

    pub mod window_switcher {
        //! This protocol will allow privileged clients to list, switch to and close other clients surfaces
        wayland_protocol_versioned!(
            "window-switcher",
            [v1],
            [
                (wl_seat, wl_seat_interface),
                (wl_surface, wl_surface_interface)
            ]
        );
    }
}
