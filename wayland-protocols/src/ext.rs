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
