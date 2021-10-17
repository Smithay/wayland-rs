//! Misc protocols
//!
//! This module contains protocols that are not clearly packaged by their maintainers,
//! often being implementation details of desktop environment, but can be used by external
//! tools for interoperability.
//!
//! Given they are not clearly packaged, the maintainers of this crate cannot guarantee
//! anything when it comes to them being up to date or the stability of their interface.
//! Pull requests for updating them will be welcomed, but we won't actively check if they
//! have received any updates.

#![cfg_attr(rustfmt, rustfmt_skip)]

pub mod gtk_primary_selection {
    //! Gtk primary selection protocol
    //!
    //! This protocol provides the ability to have a primary selection device to
    //! match that of the X server. This primary selection is a shortcut to the
    //! common clipboard selection, where text just needs to be selected in order
    //! to allow copying it elsewhere. The de facto way to perform this action
    //! is the middle mouse button, although it is not limited to this one.
    //!
    //! Clients wishing to honor primary selection should create a primary
    //! selection source and set it as the selection through
    //! `wp_primary_selection_device.set_selection` whenever the text selection
    //! changes. In order to minimize calls in pointer-driven text selection,
    //! it should happen only once after the operation finished. Similarly,
    //! a NULL source should be set when text is unselected.
    //!
    //! `wp_primary_selection_offer` objects are first announced through the
    //! `wp_primary_selection_device.data_offer` event. Immediately after this event,
    //! the primary data offer will emit `wp_primary_selection_offer.offer` events
    //! to let know of the mime types being offered.
    //!
    //! When the primary selection changes, the client with the keyboard focus
    //! will receive `wp_primary_selection_device.selection` events. Only the client
    //! with the keyboard focus will receive such events with a non-NULL
    //! `wp_primary_selection_offer`. Across keyboard focus changes, previously
    //! focused clients will receive `wp_primary_selection_device.events` with a
    //! NULL `wp_primary_selection_offer`.
    //!
    //! In order to request the primary selection data, the client must pass
    //! a recent serial pertaining to the press event that is triggering the
    //! operation, if the compositor deems the serial valid and recent, the
    //! `wp_primary_selection_source.send` event will happen in the other end
    //! to let the transfer begin. The client owning the primary selection
    //! should write the requested data, and close the file descriptor
    //! immediately.
    //!
    //! If the primary selection owner client disappeared during the transfer,
    //! the client reading the data will receive a
    //! `wp_primary_selection_device.selection` event with a NULL
    //! `wp_primary_selection_offer`, the client should take this as a hint
    //! to finish the reads related to the no longer existing offer.
    //!
    //! The primary selection owner should be checking for errors during
    //! writes, merely cancelling the ongoing transfer if any happened.

    wayland_protocol!("gtk-primary-selection", [(wl_seat, wl_seat_interface)], []);
}

#[cfg(feature = "unstable_protocols")]
pub mod zwp_input_method_v2 {
    //! Input method v2 unstable
    //! 
    //! This protocol allows applications to act as input methods for compositors.
    //!
    //! An input method context is used to manage the state of the input method.
    //!
    //! Text strings are UTF-8 encoded, their indices and lengths are in bytes.
    //!
    //! This document adheres to the RFC 2119 when using words like "must",
    //! "should", "may", etc.
    //!
    //! Warning! The protocol described in this file is experimental and
    //! backward incompatible changes may be made. Backward compatible changes
    //! may be added together with the corresponding interface version bump.
    //! Backward incompatible changes are done by bumping the version number in
    //! the protocol and interface names and resetting the interface version.
    //! Once the protocol is to be declared stable, the 'z' prefix and the
    //! version number in the protocol and interface names are removed and the
    //! interface version number is reset.

    wayland_protocol!(
        "input-method-unstable-v2",
        [
            (wl_seat, wl_seat_interface),
            (wl_surface, wl_surface_interface),
            (wl_output, wl_output_interface),
            (wl_keyboard, wl_keyboard_interface)
        ],
        [(unstable::text_input::v3, zwp_text_input_v3, zwp_text_input_v3_interface)]
    );
}

pub mod server_decoration{
    //! KDE server decoration protocol
    //!
    //! This interface allows to coordinate whether the server should create
    //! a server-side window decoration around a wl_surface representing a
    //! shell surface (wl_shell_surface or similar). By announcing support
    //! for this interface the server indicates that it supports server
    //! side decorations.
    //!
    //! Use in conjunction with zxdg_decoration_manager_v1 is undefined.
    wayland_protocol!("server-decoration", [(wl_surface, wl_seat_surface)], []);
}
