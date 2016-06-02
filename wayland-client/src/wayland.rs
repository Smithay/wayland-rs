//! The core wayland protocol
//!
//! This module contains the objects related to the core wayland protocol, you cannot
//! not use it.
//!
//! The global entry point is the `get_display()` function.
//!
//! Objects for this protocol are grouped in submodules, one for each global object
//! and the associated enums and objects it can instantiate.
//!
//! ## Surfaces
//!
//! The `wl_surface` object is the core of the protocol. You can create them
//! from the `wl_compositor` global, and they are the building blocks of your UI.
//!
//! To be displayed, a surface must have two things associated with it:
//!
//! - its contents (from a buffer or an OpenGL / Vulkan context), which also
//!   defined its dimensions
//! - its role, which defines what this surface is used for (contents of a window,
//!   icon of the curso, ...)
//!
//! A surface can only have a single role at the same time, and associating it
//! a new role without removing the previous one (by destroying the role object)
//! is a protocol error.

use std::io;

use events::{EventIterator, create_event_iterator};
use {Proxy, ProxyInternal};

use wayland_sys::client::*;

/// Objects related to the `wl_compositor` global
///
/// This global object will allow you to create `wl_surface`s, the most
/// basic building block of your application's interface.
pub mod compositor {
    pub use sys::wayland::client::{WlCompositor, WlRegion, WlSurface};
    pub use sys::wayland::client::WlSurfaceEvent;
}

/// Objects related to the `wl_data_device_manager` global
///
/// This global object will provide you with the ability to transfer data
/// from/to other wayland client applications, respectively via the
/// `wl_data_device` and `wl_data_source` objects.
pub mod data_device {
    pub use sys::wayland::client::{WlDataDevice, WlDataDeviceManager, WlDataOffer, WlDataSource};
    pub use sys::wayland::client::{WlDataDeviceEvent, WlDataOfferEvent, WlDataSourceEvent};
    pub use sys::wayland::client::{WlDataDeviceManagerDndAction};
}

/// Objects related to the `wl_output` globals
///
/// This global can be presented several times by the wayland compositor. Each of them
/// represents a single monitor-like output device of the system.
///
/// They can be added or deleted at runtine by the compositor, via events of the `wl_registry`.
pub mod output {
    pub use sys::wayland::client::WlOutput;
    pub use sys::wayland::client::WlOutputEvent;
    pub use sys::wayland::client::{WlOutputMode, WlOutputSubpixel, WlOutputTransform};
}

/// Objects related to the `wl_seat` globals
///
/// This global can be presented several times by the wayland compositor, but it will
/// in practice be very unlikely. Each of them represents an user input group of sources
/// (that can be represented as "everything that is in front of the seat of the user").
///
/// Each seat will typically handle a pointer and a keyboard, maybe a touchscreen.
///
/// They can be added or deleted at runtine by the compositor, via events of the `wl_registry`.
pub mod seat {
    pub use sys::wayland::client::{WlKeyboard, WlPointer, WlSeat, WlTouch};
    pub use sys::wayland::client::{WlKeyboardEvent, WlPointerEvent, WlSeatEvent, WlTouchEvent};
    pub use sys::wayland::client::{WlKeyboardKeyState, WlKeyboardKeymapFormat, WlPointerAxis};
    pub use sys::wayland::client::{WlPointerButtonState, WlSeatCapability, WlPointerAxisSource};
}

/// Objects related to the `wl_shell` global
///
/// This global object allows you to assign the `shell_surface` role to your surfaces, in order
/// to promote them to windows for the user (also called toplevel surfaces), render them
/// fullscreen, or as popups.
pub mod shell {
    pub use sys::wayland::client::{WlShell, WlShellSurface};
    pub use sys::wayland::client::WlShellSurfaceEvent;
    pub use sys::wayland::client::{WlShellSurfaceFullscreenMethod, WlShellSurfaceResize, WlShellSurfaceTransient};
}

/// Objects related to the `wl_shm` global
///
/// This global object allows you to create shared memory pools between your application
/// and the wayland compositor. You can then define buffers in these memory pools, which
/// you can attach to surfaces to define their contents.
pub mod shm {
    pub use sys::wayland::client::{WlBuffer, WlShm, WlShmPool};
    pub use sys::wayland::client::{WlBufferEvent, WlShmEvent};
    pub use sys::wayland::client::WlShmFormat;
}

/// Objects related to the `wl_subcompositor` global
///
/// This global allows you to assign the `subsurface` role to your surfaces, in order
/// to attach them to an other parent surface.
pub mod subcompositor {
    pub use sys::wayland::client::{WlSubcompositor, WlSubsurface};
}

pub use sys::wayland::client::{WlCallback, WlDisplay, WlRegistry};
pub use sys::wayland::client::{WlCallbackEvent, WlDisplayEvent, WlRegistryEvent};

pub use sys::wayland::client::WaylandProtocolEvent;

/// Enum representing the possible reasons why connecting to the wayland server failed
#[derive(Debug)]
pub enum ConnectError {
    /// The library was compiled with the `dlopen` feature, and the `libwayland-client.so`
    /// library could not be found at runtime
    NoWaylandLib,
    /// Any needed library was found, but the listening socket of the server could not be
    /// found.
    ///
    /// Most of the time, this means that the program was not started from a wayland session.
    NoCompositorListening
}

/// Connect to the compositor socket
///
/// Attempt to connect to a Wayland compositor according to the environment variables.
///
/// On success, returns the display object, as well as the default event iterator associated with it.
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

