//! Protocols related to window management

#![cfg_attr(rustfmt, rustfmt_skip)]

#[cfg(feature = "staging")]
pub mod activation {
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

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/xdg-activation/xdg-activation-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod decoration {
    //! This interface allows a compositor to announce support for server-side
    //! decorations.

    //! A window decoration is a set of window controls as deemed appropriate by
    //! the party managing them, such as user interface components used to move,
    //! resize and change a window's state.

    //! A client can use this protocol to request being decorated by a supporting
    //! compositor.

    //! If compositor and client do not negotiate the use of a server-side
    //! decoration using this protocol, clients continue to self-decorate as they
    //! see fit.

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/xdg-decoration/xdg-decoration-unstable-v1.xml",
            [crate::xdg::shell]
        );
    }
}

#[cfg(feature = "unstable")]
pub mod foreign {
    //! Protocol for exporting xdg surface handles
    //!
    //! This protocol specifies a way for making it possible to reference a surface
    //! of a different client. With such a reference, a client can, by using the
    //! interfaces provided by this protocol, manipulate the relationship between
    //! its own surfaces and the surface of some other client. For example, stack
    //! some of its own surface above the other clients surface.
    //!
    //! In order for a client A to get a reference of a surface of client B, client
    //! B must first export its surface using xdg_exporter.export. Upon doing this,
    //! client B will receive a handle (a unique string) that it may share with
    //! client A in some way (for example D-Bus). After client A has received the
    //! handle from client B, it may use xdg_importer.import to create a reference
    //! to the surface client B just exported. See the corresponding requests for
    //! details.
    //!
    //! A possible use case for this is out-of-process dialogs. For example when a
    //! sandboxed client without file system access needs the user to select a file
    //! on the file system, given sandbox environment support, it can export its
    //! surface, passing the exported surface handle to an unsandboxed process that
    //! can show a file browser dialog and stack it above the sandboxed client's
    //! surface.

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/xdg-foreign/xdg-foreign-unstable-v1.xml",
            []
        );
    }
    
    /// Unstable version 2
    pub mod zv2 {
        wayland_protocol!(
            "./protocols/unstable/xdg-foreign/xdg-foreign-unstable-v2.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod xdg_output {
    //! Protocol to describe output regions
    //!
    //! This protocol aims at describing outputs in a way which is more in line
    //! with the concept of an output on desktop oriented systems.
    //!
    //! Some information are more specific to the concept of an output for
    //! a desktop oriented system and may not make sense in other applications,
    //! such as IVI systems for example.
    //!
    //! Typically, the global compositor space on a desktop system is made of
    //! a contiguous or overlapping set of rectangular regions.
    //!
    //! Some of the information provided in this protocol might be identical
    //! to their counterparts already available from wl_output, in which case
    //! the information provided by this protocol should be preferred to their
    //! equivalent in wl_output. The goal is to move the desktop specific
    //! concepts (such as output location within the global compositor space,
    //! the connector name and types, etc.) out of the core wl_output protocol.

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/xdg-output/xdg-output-unstable-v1.xml",
            []
        );
    }
}

pub mod shell {
    //! XDG Shell protocol
    //!
    //! Exposes the `xdg_wm_base` global, which deprecates and replaces `wl_shell`.

    wayland_protocol!(
        "./protocols/stable/xdg-shell/xdg-shell.xml",
        []
    );
}
