#![cfg_attr(rustfmt, rustfmt_skip)]

pub mod presentation_time {
    //! Presentation time protocol
    //!
    //! Allows precise feedback on presentation timing for example for smooth video playback.

    wayland_protocol!(
        "presentation-time",
        [
            (wl_surface, wl_surface_interface),
            (wl_output, wl_output_interface)
        ]
    );
}

pub mod viewporter {
    //! Viewporter protocol
    //!
    //! Gives the capability to scale and crop surfaces, decorrelating the surface dimensions from
    //! the size of the buffer.

    wayland_protocol!(
        "viewporter",
        [
            (wl_surface, wl_surface_interface)
        ]
    );
}
