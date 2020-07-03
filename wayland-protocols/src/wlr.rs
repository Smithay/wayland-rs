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

        wayland_protocol_versioned!("wlr-data-control", [v1], [(wl_seat, wl_seat_interface)], []);
    }

    pub mod export_dmabuf {
        //! A protocol for low overhead screen content capturing
        //!
        //! An interface to capture surfaces in an efficient way by exporting DMA-BUFs.

        wayland_protocol_versioned!(
            "wlr-export-dmabuf",
            [v1],
            [(wl_output, wl_output_interface)],
            []
        );
    }

    pub mod foreign_toplevel {
        //! List and control opened apps
        //!
        //! Use for creating taskbars and docks.

        wayland_protocol_versioned!(
            "wlr-foreign-toplevel-management",
            [v1],
            [
                (wl_seat, wl_seat_interface),
                (wl_surface, wl_surface_interface),
                (wl_output, wl_output_interface)
            ],
            []
        );
    }

    pub mod gamma_control {
        //! Manage gamma tables of outputs.
        //!
        //! This protocol allows a privileged client to set the gamma tables for outputs.

        wayland_protocol_versioned!(
            "wlr-gamma-control",
            [v1],
            [(wl_output, wl_output_interface)],
            []
        );
    }

    pub mod input_inhibitor {
        //! Inhibits input events to other clients

        wayland_protocol_versioned!("wlr-input-inhibitor", [v1], [], []);
    }

    pub mod layer_shell {
        //! Layered shell protocol

        wayland_protocol_versioned!(
            "wlr-layer-shell",
            [v1],
            [(wl_output, wl_output_interface), (wl_surface, wl_surface_interface)],
            [(xdg_shell, xdg_popup, xdg_popup_interface)]
        );
    }

    pub mod output_management {
        //! Output management protocol
        //!
        //! This protocol exposes interfaces to obtain and modify output device configuration.

        wayland_protocol_versioned!(
            "wlr-output-management",
            [v1],
            [(wl_output, wl_output_interface)],
            []
        );
    }

    pub mod output_power_management {
        //! Output power management protocol
        //!
        //! This protocol allows clients to control power management modes
        //! of outputs that are currently part of the compositor space. The
        //! intent is to allow special clients like desktop shells to power
        //! down outputs when the system is idle.

        wayland_protocol_versioned!(
            "wlr-output-power-management",
            [v1],
            [(wl_output, wl_output_interface)],
            []
        );
    }

    pub mod screencopy {
        //! Screen content capturing on client buffers
        //!
        //! This protocol allows clients to ask the compositor to copy part of the
        //! screen content to a client buffer.

        wayland_protocol_versioned!(
            "wlr-screencopy",
            [v1],
            [(wl_buffer, wl_buffer_interface), (wl_output, wl_output_interface), (wl_shm, wl_shm_interface)],
            []
        );
    }

    pub mod virtual_pointer {
        //! Virtual pointer protocol
        //!
        //! This protocol allows clients to emulate a physical pointer device. The
        //! requests are mostly mirror opposites of those specified in wl_pointer.

        wayland_protocol_versioned!(
            "wlr-virtual-pointer",
            [v1],
            [(wl_seat, wl_seat_interface), (wl_output, wl_output_interface), (wl_pointer, wl_pointer_interface)],
            []
        );
    }


}
