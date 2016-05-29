//! The core wayland protocol
//!
//! This module contains the objects related to the core wayland protocol, you cannot
//! not use it.
//!
//! The global entry point is the `get_display()` function.

use std::io;

use events::{EventIterator, create_event_iterator};
use {Proxy, ProxyInternal};

use wayland_sys::client::*;

pub mod compositor {
    pub use sys::wayland::client::{WlCompositor, WlRegion, WlSurface};
    pub use sys::wayland::client::WlSurfaceEvent;
}

pub mod data_device {
    pub use sys::wayland::client::{WlDataDevice, WlDataDeviceManager, WlDataOffer, WlDataSource};
    pub use sys::wayland::client::{WlDataDeviceEvent, WlDataOfferEvent, WlDataSourceEvent};
    pub use sys::wayland::client::{WlDataDeviceManagerDndAction};
}
pub mod output {
    pub use sys::wayland::client::WlOutput;
    pub use sys::wayland::client::WlOutputEvent;
    pub use sys::wayland::client::{WlOutputMode, WlOutputSubpixel, WlOutputTransform};
}

pub mod seat {
    pub use sys::wayland::client::{WlKeyboard, WlPointer, WlSeat, WlTouch};
    pub use sys::wayland::client::{WlKeyboardEvent, WlPointerEvent, WlSeatEvent, WlTouchEvent};
    pub use sys::wayland::client::{WlKeyboardKeyState, WlKeyboardKeymapFormat, WlPointerAxis};
    pub use sys::wayland::client::{WlPointerButtonState, WlSeatCapability, WlPointerAxisSource};
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

#[derive(Debug)]
pub enum ConnectError {
    NoWaylandLib,
    NoCompositorListening
}

/// Connect to the compositor socket
///
/// Attempt to connect to a Wayland compositor according to the environment variables.
pub fn get_display() -> Result<(WlDisplay, EventIterator), ConnectError> {
    if !::wayland_sys::client::is_lib_available() { return Err(ConnectError::NoWaylandLib) }
    let ptr = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_connect, ::std::ptr::null()) };
    if ptr.is_null() {
        Err(ConnectError::NoCompositorListening)
    } else {
        let mut display = unsafe { WlDisplay::from_ptr(ptr as *mut _) };
        let eventiter = create_event_iterator(display.ptr() as *mut wl_display, None);
        display.set_event_iterator(&eventiter);
        Ok((display, eventiter))
    }
}

impl WlDisplay {
    /// Non-blocking write to the server
    ///
    /// Will write as many pending requests as possible to the server socket. Never blocks: if not all
    /// requests coul be written, will return an io error `WouldBlock`.
    ///
    /// On success returns the number of written requests.
    pub fn flush(&self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_flush, self.ptr() as *mut _) };
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
    }

    /// Create a new EventIterator
    ///
    /// No object is by default attached to it.
    pub fn create_event_iterator(&self) -> EventIterator {
        let evq = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_create_queue, self.ptr() as *mut _) };
        create_event_iterator(self.ptr() as *mut _, Some(evq))
    }
}

impl Drop for WlDisplay {
    fn drop(&mut self) {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_disconnect, self.ptr() as *mut _) }
    }
}

