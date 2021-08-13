//! wlr-procotols extension family
//!
//! This module regroup bindings to the protocol extensions from
//! [wlr-protocols](https://github.com/swaywm/wlr-protocols).

#![cfg_attr(rustfmt, rustfmt_skip)]

#[cfg(feature = "unstable_protocols")]
pub mod unstable {
    //! Unstable protocols from wlr-protocols
    //!
    //! The protocols described in this module are experimental and
    //! backward incompatible changes may be made. Backward compatible
    //! changes may be added together with the corresponding interface
    //! version bump.
    //!
    //! Backward incompatible changes are done by bumping the version
    //! number in the protocol and interface names and resetting the
    //! interface version. Once the protocol is to be declared stable,
    //! the 'z' prefix and the version number in the protocol and
    //! interface names are removed and the interface version number is
    //! reset.

    pub mod data_control {
        //! Control data devices, particularly the clipboard.
        //!
        //! An interface to control data devices, particularly to manage the current selection and
        //! take the role of a clipboard manager.

        mod v1 {
            wayland_protocol!(
                "./wlr-protocols/unstable/wlr-data-control-unstable-v1.xml",
                []
            );
        }
    }

    pub mod export_dmabuf {
        //! A protocol for low overhead screen content capturing
        //!
        //! An interface to capture surfaces in an efficient way by exporting DMA-BUFs.

        mod v1 {
            wayland_protocol!(
                "./wlr-protocols/unstable/wlr-export-dmabuf-unstable-v1.xml",
                []
            );
        }
    }

    pub mod foreign_toplevel {
        //! List and control opened apps
        //!
        //! Use for creating taskbars and docks.

        mod v1 {
            wayland_protocol!(
                "./wlr-protocols/unstable/wlr-foreign-toplevel-management-unstable-v1.xml",
                []
            );
        }
    }

    pub mod gamma_control {
        //! Manage gamma tables of outputs.
        //!
        //! This protocol allows a privileged client to set the gamma tables for outputs.

        mod v1 {
            wayland_protocol!(
                "./wlr-protocols/unstable/wlr-gamma-control-unstable-v1.xml",
                []
            );
        }
    }

    pub mod input_inhibitor {
        //! Inhibits input events to other clients

        mod v1 {
            wayland_protocol!(
                "./wlr-protocols/unstable/wlr-input-inhibitor-unstable-v1.xml",
                []
            );
        }
    }

    pub mod layer_shell {
        //! Layered shell protocol

        mod v1 {
            wayland_protocol!(
                "./wlr-protocols/unstable/wlr-layer-shell-unstable-v1.xml",
                [crate::xdg_shell]
            );
        }
    }

    pub mod output_management {
        //! Output management protocol
        //!
        //! This protocol exposes interfaces to obtain and modify output device configuration.

        mod v1 {
            wayland_protocol!(
                "./wlr-protocols/unstable/wlr-output-management-unstable-v1.xml",
                []
            );
        }
    }

    pub mod output_power_management {
        //! Output power management protocol
        //!
        //! This protocol allows clients to control power management modes
        //! of outputs that are currently part of the compositor space. The
        //! intent is to allow special clients like desktop shells to power
        //! down outputs when the system is idle.

        mod v1 {
            wayland_protocol!(
                "./wlr-protocols/unstable/wlr-output-power-management-unstable-v1.xml",
                []
            );
        }
    }

    pub mod screencopy {
        //! Screen content capturing on client buffers
        //!
        //! This protocol allows clients to ask the compositor to copy part of the
        //! screen content to a client buffer.

        mod v1 {
            wayland_protocol!(
                "./wlr-protocols/unstable/wlr-screencopy-unstable-v1.xml",
                []
            );
        }
    }

    pub mod virtual_pointer {
        //! Virtual pointer protocol
        //!
        //! This protocol allows clients to emulate a physical pointer device. The
        //! requests are mostly mirror opposites of those specified in wl_pointer.

        mod v1 {
            wayland_protocol!(
                "./wlr-protocols/unstable/wlr-virtual-pointer-unstable-v1.xml",
                []
            );
        }
    }


}
