//! Experimental protocols

#![cfg_attr(rustfmt, rustfmt_skip)]

pub mod session_management {
    //! The xx_session_manager protocol declares interfaces necessary to
    //! allow clients to restore toplevel state from previous executions.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/experimental/xx-session-management/xx-session-management-v1.xml",
            []
        );
    }
}

pub mod input_method {
    //! This protocol allows applications to act as input methods for compositors.
    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/experimental/xx-input-method/xx-input-method-v2.xml",
            [wayland_protocols::wp::text_input::zv3]
        );
    }
}