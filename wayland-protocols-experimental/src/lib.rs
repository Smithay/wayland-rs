//! This crate provides Wayland object definitions for experimental protocol extensions.
//! 
//! This crate provides bindings for protocols that are officially evaluated,
//! but not recommended for use outside of testing.
//!
//! These bindings are built on top of the crates wayland-client and wayland-server.
//!
//! Each protocol module contains a `client` and a `server` submodules, for each side of the
//! protocol. The creation of these modules (and the dependency on the associated crate) is
//! controlled by the two cargo features `client` and `server`.

#![warn(missing_docs)]
#![forbid(improper_ctypes, unsafe_op_in_unsafe_fn)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(rustfmt, rustfmt_skip)]

#[macro_use]
mod protocol_macro;

pub mod session_management {
    //! The xx_session_manager protocol declares interfaces necessary to
    //! allow clients to restore toplevel state from previous executions.

    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/xx-session-management/xx-session-management-v1.xml",
            [wayland_protocols::xdg::shell]
        );
    }
}

pub mod input_method {
    //! This protocol allows applications to act as input methods for compositors.
    #[allow(missing_docs)]
    pub mod v1 {
        wayland_protocol!(
            "./protocols/xx-input-method/xx-input-method-v2.xml",
            [wayland_protocols::wp::text_input::zv3]
        );
    }
}