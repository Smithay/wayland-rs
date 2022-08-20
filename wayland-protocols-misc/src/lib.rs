//! This crate provides Wayland object definitions for various orphan or deprecated protocol extensions.
//! 
//! This crate provides bindings for protocols that are generally not officially supported,
//! but are *de facto* used by a non-negligible number of projets in the wayland ecosystem.
//!
//! These bindings are built on top of the crates wayland-client and wayland-server.
//!
//! Each protocol module contains a `client` and a `server` submodules, for each side of the
//! protocol. The creation of these modules (and the dependency on the associated crate) is
//! controlled by the two cargo features `client` and `server`.

#![warn(missing_docs)]
#![forbid(improper_ctypes, unsafe_op_in_unsafe_fn)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(rustfmt, rustfmt_skip)]

#[macro_use]
mod protocol_macro;

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

    wayland_protocol!("./protocols/gtk-primary-selection.xml", []);
}

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

    wayland_protocol!("./protocols/input-method-unstable-v2.xml", [wayland_protocols::wp::text_input::zv3]);
}

pub mod zwp_virtual_keyboard_v1 {
    //! Virtual keyboard v1 unstable
    //!
    //! The virtual keyboard provides an application with requests which emulate
    //! the behaviour of a physical keyboard.
    //!
    //! This interface can be used by clients on its own to provide raw input
    //! events, or it can accompany the input method protocol.

    wayland_protocol!("./protocols/virtual-keyboard-unstable-v1.xml", []);
}

pub mod server_decoration {
    //! KDE server decoration protocol
    //!
    //! This interface allows to coordinate whether the server should create
    //! a server-side window decoration around a wl_surface representing a
    //! shell surface (wl_shell_surface or similar). By announcing support
    //! for this interface the server indicates that it supports server
    //! side decorations.
    //!
    //! Use in conjunction with zxdg_decoration_manager_v1 is undefined.

    wayland_protocol!("./protocols/server-decoration.xml", []);
}
