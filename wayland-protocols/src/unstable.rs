#![cfg_attr(rustfmt, rustfmt_skip)]

pub mod fullscreen_shell {
    wayland_protocol!("fullscreen-shell",
                      (wl_surface, wl_surface_interface),
                      (wl_output, wl_output_interface));
}

pub mod idle_inhibit {
    wayland_protocol!("idle-inhibit", (wl_surface, wl_surface_interface));
}


pub mod input_method {
    wayland_protocol!("input-method",
                      (wl_surface, wl_surface_interface),
                      (wl_output, wl_output_interface),
                      (wl_keyboard, wl_keyboard_interface));
}

pub mod linux_dmabuf {
    wayland_protocol!("linux-dmabuf", (wl_buffer, wl_buffer_interface));
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

    wayland_protocol!("pointer-constraints",
                      (wl_surface, wl_surface_interface),
                      (wl_pointer, wl_pointer_interface),
                      (wl_region, wl_region_interface));
}

pub mod pointer_gestures {
    wayland_protocol!("pointer-gestures",
                      (wl_surface, wl_surface_interface),
                      (wl_pointer, wl_pointer_interface));
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

    wayland_protocol!("relative-pointer", (wl_pointer, wl_pointer_interface));
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

    wayland_protocol!("tablet",
                      (wl_seat, wl_seat_interface),
                      (wl_surface, wl_surface_interface));
}

pub mod text_input {
    wayland_protocol!("text-input",
                      (wl_seat, wl_seat_interface),
                      (wl_surface, wl_surface_interface));
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

    wayland_protocol!("xdg-foreign", (wl_surface, wl_surface_interface));
}

pub mod xdg_shell {
    wayland_protocol!("xdg-shell",
                      (wl_surface, wl_surface_interface),
                      (wl_output, wl_output_interface),
                      (wl_seat, wl_seat_interface));
}
