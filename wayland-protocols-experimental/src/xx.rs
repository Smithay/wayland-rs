//! Experimental protocols.
//!
//! Those protocols are ready for wider testing, but they may change often and in incompatible ways, or disappear altogether. Software using them should implement the latest version only.
//!
//! It is expected that they eventually get promoted to staging or removed.


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