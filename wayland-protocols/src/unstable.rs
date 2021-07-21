//! Unstable protocols from wayland-protocols
//!
//! The protocols described in this module are experimental and
//! provide no guarantee of forward support. They may be abandoned
//! or never widely implemented.
//!
//! Backward compatible changes may be added together with the
//! corresponding interface version bump.
//!
//! Backward incompatible changes are done by bumping the version
//! number in the protocol and interface names and resetting the
//! interface version. Once the protocol is to be declared stable,
//! the 'z' prefix and the version number in the protocol and
//! interface names are removed and the interface version number is
//! reset.

#![cfg_attr(rustfmt, rustfmt_skip)]

pub mod fullscreen_shell {
    //! Fullscreen shell protocol

    wayland_protocol_versioned!(
        "fullscreen-shell",
        [v1],
        [(wl_surface, wl_surface_interface), (wl_output, wl_output_interface)],
        []
    );
}

pub mod idle_inhibit {
    //! Screensaver inhibition protocol

    wayland_protocol_versioned!("idle-inhibit", [v1], [(wl_surface, wl_surface_interface)], []);
}

pub mod input_method {
    //! Input method protocol

    wayland_protocol_versioned!(
        "input-method",
        [v1],
        [
            (wl_surface, wl_surface_interface),
            (wl_output, wl_output_interface),
            (wl_keyboard, wl_keyboard_interface)
        ],
        []
    );
}

pub mod input_timestamps {
    //! Input timestamps protocol

    wayland_protocol_versioned!(
        "input-timestamps",
        [v1],
        [
            (wl_keyboard, wl_keyboard_interface),
            (wl_pointer, wl_pointer_interface),
            (wl_touch, wl_touch_interface)
        ],
        []
    );
}

pub mod keyboard_shortcuts_inhibit {
    //! Protocol for inhibiting the compositor keyboard shortcuts
    //!
    //! This protocol specifies a way for a client to request the compositor
    //! to ignore its own keyboard shortcuts for a given seat, so that all
    //! key events from that seat get forwarded to a surface.

    wayland_protocol_versioned!(
        "keyboard-shortcuts-inhibit",
        [v1],
        [(wl_seat, wl_seat_interface), (wl_surface, wl_surface_interface)],
        []
    );
}

pub mod linux_dmabuf {
    //! Linux DMA-BUF protocol

    wayland_protocol_versioned!("linux-dmabuf", [v1], [(wl_buffer, wl_buffer_interface)], []);
}

pub mod linux_explicit_synchronization {
    //! Linux explicit synchronization protocol

    wayland_protocol_versioned!(
        "linux-explicit-synchronization",
        [v1],
        [(wl_surface, wl_surface_interface)],
        []
    );
}

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

    wayland_protocol_versioned!(
        "pointer-constraints",
        [v1],
        [
            (wl_surface, wl_surface_interface),
            (wl_pointer, wl_pointer_interface),
            (wl_region, wl_region_interface)
        ],
        []
    );
}

pub mod pointer_gestures {
    //! Pointer gestures protocol

    wayland_protocol_versioned!(
        "pointer-gestures",
        [v1],
        [(wl_surface, wl_surface_interface), (wl_pointer, wl_pointer_interface)],
        []
    );
}

pub mod primary_selection {
    //! Primary selection protocol

    wayland_protocol_versioned!("primary-selection", [v1], [(wl_seat, wl_seat_interface)], []);
}

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

    wayland_protocol_versioned!("relative-pointer", [v1], [(wl_pointer, wl_pointer_interface)], []);
}

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

    wayland_protocol_versioned!(
        "tablet",
        [v1, v2],
        [(wl_seat, wl_seat_interface), (wl_surface, wl_surface_interface)],
        []
    );
}

pub mod text_input {
    //! Text input protocol

    wayland_protocol_versioned!(
        "text-input",
        [v1, v3],
        [(wl_seat, wl_seat_interface), (wl_surface, wl_surface_interface)],
        []
    );
}

pub mod xdg_decoration {
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

    wayland_protocol_versioned!(
        "xdg-decoration",
        [v1],
        [],
        [(xdg_shell, xdg_toplevel, xdg_toplevel_interface)]
    );
}

pub mod xdg_foreign {
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

    wayland_protocol_versioned!("xdg-foreign", [v1, v2], [(wl_surface, wl_surface_interface)], []);
}

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

    wayland_protocol_versioned!("xdg-output", [v1], [(wl_output, wl_output_interface)], []);
}

pub mod xdg_shell {
    //! XDG Shell protocol
    //!
    //! These are the old, unstable versions of the now stable XDG Shell protocol.
    //!
    //! They remain here for compatibility reasons, allowing you to support older
    //! clients/server not yet implementing the new protocol.

    wayland_protocol_versioned!(
        "xdg-shell",
        [v5, v6],
        [
            (wl_surface, wl_surface_interface),
            (wl_output, wl_output_interface),
            (wl_seat, wl_seat_interface)
        ],
        []
    );
}

pub mod xwayland_keyboard_grab {
    //! Protocol for grabbing the keyboard from Xwayland
    //!
    //! This protocol is application-specific to meet the needs of the X11
    //! protocol through Xwayland. It provides a way for Xwayland to request
    //! all keyboard events to be forwarded to a surface even when the
    //! surface does not have keyboard focus.
    //!
    //! In the X11 protocol, a client may request an "active grab" on the
    //! keyboard. On success, all key events are reported only to the
    //! grabbing X11 client. For details, see XGrabKeyboard(3).
    //!
    //! The core Wayland protocol does not have a notion of an active
    //! keyboard grab. When running in Xwayland, X11 applications may
    //! acquire an active grab inside Xwayland but that cannot be translated
    //! to the Wayland compositor who may set the input focus to some other
    //! surface. In doing so, it breaks the X11 client assumption that all
    //! key events are reported to the grabbing client.
    //!
    //! This protocol specifies a way for Xwayland to request all keyboard
    //! be directed to the given surface. The protocol does not guarantee
    //! that the compositor will honor this request and it does not
    //! prescribe user interfaces on how to handle the respond. For example,
    //! a compositor may inform the user that all key events are now
    //! forwarded to the given client surface, or it may ask the user for
    //! permission to do so.
    //!
    //! Compositors are required to restrict access to this application
    //! specific protocol to Xwayland alone.

    wayland_protocol_versioned!(
        "xwayland-keyboard-grab",
        [v1],
        [(wl_seat, wl_seat_interface), (wl_surface, wl_surface_interface)],
        []
    );
}
