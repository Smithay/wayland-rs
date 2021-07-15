//! Staging protocols from wayland-protocols
//!
//! The protocols described this module are in the staging process and will soon be made part of a
//! release. Staging protocols are guaranteed to have no backwards incompatible changes introduced.
//!
//! These protocols are ready for wider adoption and clients and compositors are encouraged to
//! implement staging protocol extensions where a protocol's functionality is desired.
//!
//! Although these protocols should be stable, the protocols may be completely replaced in a new
//! major version or with a completely different protocol.

#![cfg_attr(rustfmt, rustfmt_skip)]

pub mod xdg_activation {
    //! The way for a client to pass focus to another toplevel is as follows.
    //!
    //! The client that intends to activate another toplevel uses the
    //! xdg_activation_v1.get_activation_token request to get an activation token.
    //! This token is then passed to the client to be activated through a separate
    //! band of communication. The client to be activated will then pass the token
    //! it received to the xdg_activation_v1.activate request. The compositor can
    //! then use this token to decide how to react to the activation request.
    //!
    //! The token the activating client gets may be ineffective either already at
    //! the time it receives it, for example if it was not focused, for focus
    //! stealing prevention. The activating client will have no way to discover
    //! the validity of the token, and may still forward it to the to be activated
    //! client.
    //!
    //! The created activation token may optionally get information attached to it
    //! that can be used by the compositor to identify the application that we
    //! intend to activate. This can for example be used to display a visual hint
    //! about what application is being started.

    wayland_protocol_versioned!(
        "xdg-activation",
        [v1],
        [(wl_seat, wl_seat_interface), (wl_surface, wl_surface_interface)],
        []
    );
}
