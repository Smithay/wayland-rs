//! The core wayland protocol
//!
//! This module contains the objects related to the core wayland protocol, you cannot
//! not use it.

pub use sys::wayland::server::WaylandProtocolRequest;

pub mod compositor {
    pub use sys::wayland::server::{WlCompositor, WlRegion, WlSurface};
    pub use sys::wayland::server::{WlCompositorRequest, WlSurfaceRequest, WlRegionRequest};
}

pub mod data_device {
    pub use sys::wayland::server::{WlDataDevice, WlDataDeviceManager, WlDataOffer, WlDataSource};
    pub use sys::wayland::server::{WlDataDeviceRequest, WlDataDeviceManagerRequest, WlDataOfferRequest, WlDataSourceRequest};
    pub use sys::wayland::server::{WlDataDeviceManagerDndAction};
}
pub mod output {
    pub use sys::wayland::server::WlOutput;
    pub use sys::wayland::server::{WlOutputMode, WlOutputSubpixel, WlOutputTransform};
}

pub mod seat {
    pub use sys::wayland::server::{WlKeyboard, WlPointer, WlSeat, WlTouch};
    pub use sys::wayland::server::{WlKeyboardRequest, WlPointerRequest, WlSeatRequest, WlTouchRequest};
    pub use sys::wayland::server::{WlKeyboardKeyState, WlKeyboardKeymapFormat, WlPointerAxis};
    pub use sys::wayland::server::{WlPointerButtonState, WlSeatCapability, WlPointerAxisSource};
}

pub mod shell {
    pub use sys::wayland::server::{WlShell, WlShellSurface};
    pub use sys::wayland::server::{WlShellRequest, WlShellSurfaceRequest};
    pub use sys::wayland::server::{WlShellSurfaceFullscreenMethod, WlShellSurfaceResize, WlShellSurfaceTransient};
}

pub mod shm {
    pub use sys::wayland::server::{WlBuffer, WlShm, WlShmPool};
    pub use sys::wayland::server::{WlBufferRequest, WlShmRequest, WlShmPoolRequest};
    pub use sys::wayland::server::WlShmFormat;
}

pub mod subcompositor {
    pub use sys::wayland::server::{WlSubcompositor, WlSubsurface};
}

protocol_globals!(Wayland,WaylandProtocolGlobalInstance,
    Compositor => ::wayland::compositor::WlCompositor,
    DataDeviceManager => ::wayland::data_device::WlDataDeviceManager,
    Output => ::wayland::output::WlOutput,
    Seat => ::wayland::seat::WlSeat,
    Shell => ::wayland::shell::WlShell,
    Shm => ::wayland::shm::WlShm,
    SubCompositor => ::wayland::subcompositor::WlSubcompositor
);
