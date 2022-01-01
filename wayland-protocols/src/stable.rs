#![cfg_attr(rustfmt, rustfmt_skip)]

pub mod presentation_time {
    //! Presentation time protocol
    //!
    //! Allows precise feedback on presentation timing, for example for smooth video playback.

    wayland_protocol!(
        "./protocols/stable/presentation-time/presentation-time.xml",
        []
    );
}

pub mod xdg_shell {
    //! XDG Shell protocol
    //!
    //! Exposes the `xdg_wm_base` global, which deprecates and replaces `wl_shell`.

    wayland_protocol!(
        "./protocols/stable/xdg-shell/xdg-shell.xml",
        []
    );
}

pub mod viewporter {
    //! Viewporter protocol
    //!
    //! Provides the capability of scaling and cropping surfaces, decorrelating the surface
    //! dimensions from the size of the buffer.

    wayland_protocol!("./protocols/stable/viewporter/viewporter.xml", []);
}
