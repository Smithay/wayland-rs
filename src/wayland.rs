pub mod compositor {
    pub use sys::wayland::client::{WlCompositor, WlRegion, WlSurface};
    pub use sys::wayland::client::WlSurfaceEvent;
}

pub mod data_device {
    pub use sys::wayland::client::{WlDataDevice, WlDataDeviceManager, WlDataOffer, WlDataSource};
    pub use sys::wayland::client::{WlDataDeviceEvent, WlDataOfferEvent, WlDataSourceEvent};
}
pub mod output {
    pub use sys::wayland::client::WlOutput;
    pub use sys::wayland::client::WlOutputEvent;
    pub use sys::wayland::client::{WlOutputMode, WlOutputSubpixel, WlOutputTransform};
}

pub mod seat {
    pub use sys::wayland::client::{WlKeyboard, WlPointer, WlSeat, WlTouch};
    pub use sys::wayland::client::{WlKeyboardEvent, WlPointerEvent, WlSeatEvent, WlTouchEvent};
    pub use sys::wayland::client::{WlKeyboardKeyState, WlKeyboardKeymapFormat, WlPointerAxis, WlPointerButtonState, WlSeatCapability};
}

pub mod shell {
    pub use sys::wayland::client::{WlShell, WlShellSurface};
    pub use sys::wayland::client::WlShellSurfaceEvent;
    pub use sys::wayland::client::{WlShellSurfaceFullscreenMethod, WlShellSurfaceResize, WlShellSurfaceTransient};
}

pub mod shm {
    pub use sys::wayland::client::{WlBuffer, WlShm, WlShmPool};
    pub use sys::wayland::client::{WlBufferEvent, WlShmEvent};
    pub use sys::wayland::client::WlShmFormat;
}

pub mod subcompositor {
    pub use sys::wayland::client::{WlSubcompositor, WlSubsurface};
}

pub use sys::wayland::client::{WlCallback, WlDisplay, WlRegistry};
pub use sys::wayland::client::{WlCallbackEvent, WlDisplayEvent, WlRegistryEvent};