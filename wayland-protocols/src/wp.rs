//! Generic wayland protocols

#![cfg_attr(rustfmt, rustfmt_skip)]

#[cfg(feature = "staging")]
pub mod content_type {
    //! This protocol allows a client to describe the kind of content a surface
    //! will display, to allow the compositor to optimize its behavior for it.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/content-type/content-type-v1.xml",
            []
        );
    }
}

#[cfg(feature = "staging")]
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

#[cfg(feature = "staging")]
pub mod tearing_control {
    //! This protocol provides a way for clients to indicate whether
    //! or not their content is suitable for this kind of presentation.
    //!
    //! For some use cases like games or drawing tablets it can make sense to reduce
    //! latency by accepting tearing with the use of asynchronous page flips.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/tearing-control/tearing-control-v1.xml",
            []
        );
    }
}

#[cfg(feature = "staging")]
pub mod fractional_scale {
    //! This protocol allows a compositor to suggest for surfaces to render at
    //! fractional scales.
    //!
    //! A client can submit scaled content by utilizing wp_viewport. This is done by
    //! creating a wp_viewport object for the surface and setting the destination
    //! rectangle to the surface size before the scale factor is applied.
    //!
    //! The buffer size is calculated by multiplying the surface size by the
    //! intended scale.
    //!
    //! The wl_surface buffer scale should remain set to 1.
    //!
    //! If a surface has a surface-local size of 100 px by 50 px and wishes to
    //! submit buffers with a scale of 1.5, then a buffer of 150px by 75 px should
    //! be used and the wp_viewport destination rectangle should be 100 px by 50 px.
    //!
    //! For toplevel surfaces, the size is rounded halfway away from zero. The
    //! rounding algorithm for subsurface position and size is not defined.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/fractional-scale/fractional-scale-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod fullscreen_shell {
    //! Fullscreen shell protocol

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/fullscreen-shell/fullscreen-shell-unstable-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod idle_inhibit {
    //! Screensaver inhibition protocol

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/idle-inhibit/idle-inhibit-unstable-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod input_method {
    //! Input method protocol

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/input-method/input-method-unstable-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod input_timestamps {
    //! Input timestamps protocol

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/input-timestamps/input-timestamps-unstable-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod keyboard_shortcuts_inhibit {
    //! Protocol for inhibiting the compositor keyboard shortcuts
    //!
    //! This protocol specifies a way for a client to request the compositor
    //! to ignore its own keyboard shortcuts for a given seat, so that all
    //! key events from that seat get forwarded to a surface.

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/keyboard-shortcuts-inhibit/keyboard-shortcuts-inhibit-unstable-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod linux_dmabuf {
    //! Linux DMA-BUF protocol

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/linux-dmabuf/linux-dmabuf-unstable-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod linux_explicit_synchronization {
    //! Linux explicit synchronization protocol

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/linux-explicit-synchronization/linux-explicit-synchronization-unstable-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod pointer_constraints {
    //! protocol for constraining pointer motions
    //!
    //! This protocol specifies a set of interfaces used for adding constraints to
    //! the motion of a pointer. Possible constraints include confining pointer
    //! motions to a given region, or locking it to its current position.
    //!
    //! In order to constrain the pointer, a client must first bind the global
    //! interface "wp_pointer_constraints" which, if a compositor supports pointer
    //! constraints, is exposed by the registry. Using the bound global object, the
    //! client uses the request that corresponds to the type of constraint it wants
    //! to make. See wp_pointer_constraints for more details.

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/pointer-constraints/pointer-constraints-unstable-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod pointer_gestures {
    //! Pointer gestures protocol

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/pointer-gestures/pointer-gestures-unstable-v1.xml",
            []
        );
    }
}

pub mod presentation_time {
    //! Presentation time protocol
    //!
    //! Allows precise feedback on presentation timing, for example for smooth video playback.

    wayland_protocol!(
        "./protocols/stable/presentation-time/presentation-time.xml",
        []
    );
}

#[cfg(feature = "unstable")]
pub mod primary_selection {
    //! Primary selection protocol

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/primary-selection/primary-selection-unstable-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod relative_pointer {
    //! protocol for relative pointer motion events
    //!
    //! This protocol specifies a set of interfaces used for making clients able to
    //! receive relative pointer events not obstructed by barriers (such as the
    //! monitor edge or other pointer barriers).
    //!
    //! To start receiving relative pointer events, a client must first bind the
    //! global interface "wp_relative_pointer_manager" which, if a compositor
    //! supports relative pointer motion events, is exposed by the registry. After
    //! having created the relative pointer manager proxy object, the client uses
    //! it to create the actual relative pointer object using the
    //! "get_relative_pointer" request given a wl_pointer. The relative pointer
    //! motion events will then, when applicable, be transmitted via the proxy of
    //! the newly created relative pointer object. See the documentation of the
    //! relative pointer interface for more details.

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/relative-pointer/relative-pointer-unstable-v1.xml",
            []
        );
    }
}

#[cfg(feature = "staging")]
pub mod single_pixel_buffer {
    //! This protocol extension allows clients to create single-pixel buffers.
    //!
    //! Compositors supporting this protocol extension should also support the
    //! viewporter protocol extension. Clients may use viewporter to scale a
    //! single-pixel buffer to a desired size.

    /// Version 1
    pub mod v1 {
        wayland_protocol!(
            "./protocols/staging/single-pixel-buffer/single-pixel-buffer-v1.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod tablet {
    //! Wayland protocol for graphics tablets
    //!
    //! This description provides a high-level overview of the interplay between
    //! the interfaces defined this protocol. For details, see the protocol
    //! specification.
    //!
    //! More than one tablet may exist, and device-specifics matter. Tablets are
    //! not represented by a single virtual device like wl_pointer. A client
    //! binds to the tablet manager object which is just a proxy object. From
    //! that, the client requests wp_tablet_manager.get_tablet_seat(wl_seat)
    //! and that returns the actual interface that has all the tablets. With
    //! this indirection, we can avoid merging wp_tablet into the actual Wayland
    //! protocol, a long-term benefit.
    //!
    //! The wp_tablet_seat sends a "tablet added" event for each tablet
    //! connected. That event is followed by descriptive events about the
    //! hardware; currently that includes events for name, vid/pid and
    //! a wp_tablet.path event that describes a local path. This path can be
    //! used to uniquely identify a tablet or get more information through
    //! libwacom. Emulated or nested tablets can skip any of those, e.g. a
    //! virtual tablet may not have a vid/pid. The sequence of descriptive
    //! events is terminated by a wp_tablet.done event to signal that a client
    //! may now finalize any initialization for that tablet.
    //!
    //! Events from tablets require a tool in proximity. Tools are also managed
    //! by the tablet seat; a "tool added" event is sent whenever a tool is new
    //! to the compositor. That event is followed by a number of descriptive
    //! events about the hardware; currently that includes capabilities,
    //! hardware id and serial number, and tool type. Similar to the tablet
    //! interface, a wp_tablet_tool.done event is sent to terminate that initial
    //! sequence.
    //!
    //! Any event from a tool happens on the wp_tablet_tool interface. When the
    //! tool gets into proximity of the tablet, a proximity_in event is sent on
    //! the wp_tablet_tool interface, listing the tablet and the surface. That
    //! event is followed by a motion event with the coordinates. After that,
    //! it's the usual motion, axis, button, etc. events. The protocol's
    //! serialisation means events are grouped by wp_tablet_tool.frame events.
    //!
    //! Two special events (that don't exist in X) are down and up. They signal
    //! "tip touching the surface". For tablets without real proximity
    //! detection, the sequence is: proximity_in, motion, down, frame.
    //!
    //! When the tool leaves proximity, a proximity_out event is sent. If any
    //! button is still down, a button release event is sent before this
    //! proximity event. These button events are sent in the same frame as the
    //! proximity event to signal to the client that the buttons were held when
    //! the tool left proximity.
    //!
    //! If the tool moves out of the surface but stays in proximity (i.e.
    //! between windows), compositor-specific grab policies apply. This usually
    //! means that the proximity-out is delayed until all buttons are released.
    //!
    //! Moving a tool physically from one tablet to the other has no real effect
    //! on the protocol, since we already have the tool object from the "tool
    //! added" event. All the information is already there and the proximity
    //! events on both tablets are all a client needs to reconstruct what
    //! happened.
    //!
    //! Some extra axes are normalized, i.e. the client knows the range as
    //! specified in the protocol (e.g. [0, 65535]), the granularity however is
    //! unknown. The current normalized axes are pressure, distance, and slider.
    //!
    //! Other extra axes are in physical units as specified in the protocol.
    //! The current extra axes with physical units are tilt, rotation and
    //! wheel rotation.
    //!
    //! Since tablets work independently of the pointer controlled by the mouse,
    //! the focus handling is independent too and controlled by proximity.
    //! The wp_tablet_tool.set_cursor request sets a tool-specific cursor.
    //! This cursor surface may be the same as the mouse cursor, and it may be
    //! the same across tools but it is possible to be more fine-grained. For
    //! example, a client may set different cursors for the pen and eraser.
    //!
    //! Tools are generally independent of tablets and it is
    //! compositor-specific policy when a tool can be removed. Common approaches
    //! will likely include some form of removing a tool when all tablets the
    //! tool was used on are removed.

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/tablet/tablet-unstable-v1.xml",
            []
        );
    }

    /// Unstable version 2
    pub mod zv2 {
        wayland_protocol!(
            "./protocols/unstable/tablet/tablet-unstable-v2.xml",
            []
        );
    }
}

#[cfg(feature = "unstable")]
pub mod text_input {
    //! Text input protocol

    /// Unstable version 1
    pub mod zv1 {
        wayland_protocol!(
            "./protocols/unstable/text-input/text-input-unstable-v1.xml",
            []
        );
    }

    /// Unstable version 3
    pub mod zv3 {
        wayland_protocol!(
            "./protocols/unstable/text-input/text-input-unstable-v3.xml",
            []
        );
    }
}

pub mod viewporter {
    //! Viewporter protocol
    //!
    //! Provides the capability of scaling and cropping surfaces, decorrelating the surface
    //! dimensions from the size of the buffer.

    wayland_protocol!("./protocols/stable/viewporter/viewporter.xml", []);
}
