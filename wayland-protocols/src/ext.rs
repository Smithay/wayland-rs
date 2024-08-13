//! Miscellaneous protocols

#![cfg_attr(rustfmt, rustfmt_skip)]

#[cfg(feature = "staging")]
pub mod idle_notify {
    //! This protocol allows clients to monitor user idle status.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/ext-idle-notify/ext-idle-notify-v1.xml",
            []
        );
    }
}

#[cfg(feature = "staging")]
pub mod session_lock {
    //! This protocol allows for a privileged Wayland client to lock the session
    //! and display arbitrary graphics while the session is locked.
    //!
    //! The compositor may choose to restrict this protocol to a special client
    //! launched by the compositor itself or expose it to all privileged clients,
    //! this is compositor policy.
    //!
    //! The client is responsible for performing authentication and informing the
    //! compositor when the session should be unlocked. If the client dies while
    //! the session is locked the session remains locked, possibly permanently
    //! depending on compositor policy.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/ext-session-lock/ext-session-lock-v1.xml",
            []
        );
    }
}

#[cfg(feature = "staging")]
pub mod foreign_toplevel_list {
    //! The purpose of this protocol is to provide protocol object handles for toplevels, possibly
    //! originating from another client.
    //!
    //! This protocol is intentionally minimalistic and expects additional functionality (e.g.
    //! creating a screencopy source from a toplevel handle, getting information about the state of
    //! the toplevel) to be implemented in extension protocols.
    //!
    //! The compositor may choose to restrict this protocol to a special client launched by the
    //! compositor itself or expose it to all clients, this is compositor policy.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/ext-foreign-toplevel-list/ext-foreign-toplevel-list-v1.xml",
            []
        );
    }
}

#[cfg(feature = "staging")]
pub mod transient_seat {
    //! The transient seat protocol can be used by privileged clients to create
    //! independent seats that will be removed from the compositor when the client
    //! destroys its transient seat.
    //!
    //! This protocol is intended for use with virtual input protocols such as
    //! "virtual_keyboard_unstable_v1" or "wlr_virtual_pointer_unstable_v1", both
    //! of which allow the user to select a seat.
    //!
    //! The "wl_seat" global created by this protocol does not generate input events
    //! on its own, or have any capabilities except those assigned to it by other
    //! protocol extensions, such as the ones mentioned above.
    //!
    //! For example, a remote desktop server can create a seat with virtual inputs
    //! for each remote user by following these steps for each new connection:
    //!  * Create a transient seat
    //!  * Wait for the transient seat to be created
    //!  * Locate a "wl_seat" global with a matching name
    //!  * Create virtual inputs using the resulting "wl_seat" global
    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/ext-transient-seat/ext-transient-seat-v1.xml",
            []
        );
    }

}


#[cfg(feature = "staging")]
pub mod image_capture_source {
    //! This protocol serves as an intermediary between capturing protocols and
    //! potential image capture sources such as outputs and toplevels.
    //!
    //! This protocol may be extended to support more image capture sources in the
    //! future, thereby adding those image capture sources to other protocols that
    //! use the image capture source object without having to modify those
    //! protocols.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/ext-image-capture-source/ext-image-capture-source-v1.xml",
            [crate::ext::foreign_toplevel_list::v1, crate::xdg::shell]
        );
    }
}

#[cfg(feature = "staging")]
pub mod image_copy_capture {
    //! This protocol allows clients to ask the compositor to capture image sources
    //! such as outputs and toplevels into user submitted buffers.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/ext-image-copy-capture/ext-image-copy-capture-v1.xml",
            [crate::ext::image_capture_source::v1, crate::xdg::shell]
        );
    }
}
