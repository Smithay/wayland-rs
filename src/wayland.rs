use Proxy;

use abi::client::*;

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

pub use sys::wayland::client::WaylandProtocolEvent;

pub fn get_display() -> Option<WlDisplay> {
    if !::abi::client::is_lib_available() { return None }
    let ptr = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_connect, ::std::ptr::null()) };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { WlDisplay::from_ptr(ptr as *mut _) })
    }
}

impl WlDisplay {
    pub fn sync_roundtrip(&mut self) -> i32 {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_roundtrip, self.ptr() as *mut _) }
    }

    pub fn dispatch(&mut self) -> i32 {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch, self.ptr() as *mut _) }
    }

    pub fn dispatch_pending(&mut self) -> i32 {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch_pending, self.ptr() as *mut _) }
    }

    pub fn flush(&mut self) -> i32 {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_flush, self.ptr() as *mut _) }
    }
}

impl Drop for WlDisplay {
    fn drop(&mut self) {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_disconnect, self.ptr() as *mut _) }
    }
}