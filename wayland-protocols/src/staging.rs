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

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/xdg-activation/xdg-activation-v1.xml",
            []
        );
    }
}

pub mod ext_session_lock {
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

pub mod drm_lease {
    //! This protocol is used by Wayland compositors which act as Direct
    //! Renderering Manager (DRM) masters to lease DRM resources to Wayland
    //! clients.
    //!
    //! The compositor will advertise one wp_drm_lease_device_v1 global for each
    //! DRM node. Some time after a client binds to the wp_drm_lease_device_v1
    //! global, the compositor will send a drm_fd event followed by zero, one or
    //! more connector events. After all currently available connectors have been
    //! sent, the compositor will send a wp_drm_lease_device_v1.done event.
    //!
    //! When the list of connectors available for lease changes the compositor
    //! will send wp_drm_lease_device_v1.connector events for added connectors and
    //! wp_drm_lease_connector_v1.withdrawn events for removed connectors,
    //! followed by a wp_drm_lease_device_v1.done event.
    //!
    //! The compositor will indicate when a device is gone by removing the global
    //! via a wl_registry.global_remove event. Upon receiving this event, the
    //! client should destroy any matching wp_drm_lease_device_v1 object.
    //!
    //! To destroy a wp_drm_lease_device_v1 object, the client must first issue
    //! a release request. Upon receiving this request, the compositor will
    //! immediately send a released event and destroy the object. The client must
    //! continue to process and discard drm_fd and connector events until it
    //! receives the released event. Upon receiving the released event, the
    //! client can safely cleanup any client-side resources.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/drm-lease/drm-lease-v1.xml",
            []
        );
    }
}
