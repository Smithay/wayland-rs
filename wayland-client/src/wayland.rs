//! The core wayland protocol
//!
//! This module contains the objects related to the core wayland protocol, you cannot
//! not use it.
//!
//! The global entry point is the `get_display()` function.

use std::io;

use Proxy;

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

pub fn get_display() -> Option<WlDisplay> {
    if !::wayland_sys::client::is_lib_available() { return None }
    let ptr = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_connect, ::std::ptr::null()) };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { WlDisplay::from_ptr(ptr as *mut _) })
    }
}

impl WlDisplay {
    /// Synchronous roundtrip
    ///
    /// This call will cause a synchonous roundtrip with the wayland server. I will block until all
    /// pending requests are send to the server and it has processed all of them and send the
    /// appropriate events.
    ///
    /// On success returns the number of dispatched events.
    pub fn sync_roundtrip(&mut self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_roundtrip, self.ptr() as *mut _) };
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
    }

    /// Blocking dispatch
    ///
    /// Will dispatch all pending events from the internal buffer to the events iterators.
    /// If the buffer was empty, will read new events from the server socket, blocking if necessary.
    ///
    /// On success returns the number of dispatched events.
    pub fn dispatch(&mut self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch, self.ptr() as *mut _) };
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
    }

    /// Non-blocking dispatch
    ///
    /// Will dispatch all pending events from the internal buffer to the events iterators.
    /// Will not try to read events from the server socket, hence never blocks.
    ///
    /// On success returns the number of dispatched events.
    pub fn dispatch_pending(&self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_dispatch_pending, self.ptr() as *mut _) };
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
    }

    /// Prepare an conccurent read
    ///
    /// Will declare your intention to read events from the server socket. Contrarily to `dispatch()`
    /// or `sync_roundtrip()`, this method can be called several times conccurently.
    ///
    /// Will return `None` if there are still some events awaiting dispatch. In this case, you need
    /// to call `dispatch_pending()` before calling this method again.
    ///
    /// As long as the returned guard is in scope, no events can be dispatched to any event iterator.
    ///
    /// The guard can then be destroyed by two means:
    ///
    ///  - Calling its `cancel()` method (or letting it go out of scope): the read intention will
    ///    be cancelled
    ///  - Calling its `read_events()` method: will block until all existing guards are destroyed
    ///    by one of these methods, then events will be read and all blocked `read_events()` calls
    ///    will return.
    ///
    /// This call will otherwise not block on the server socket if it is empty, and return
    /// an io error `WouldBlock` in such cases.
    pub fn prepare_read(&self) -> Option<ReadEventsGuard> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_prepare_read, self.ptr() as *mut _) };
        if ret >= 0 { Some(ReadEventsGuard { display: self.ptr() as *mut _ }) } else { None }
    }

    /// Non-blocking write to the server
    ///
    /// Will write as many requests as possible to the server socket. Never blocks: if not all
    /// requests coul be written, will return an io error `WouldBlock`.
    ///
    /// On success returns the number of written requests.
    pub fn flush(&self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_flush, self.ptr() as *mut _) };
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
    }
}

impl Drop for WlDisplay {
    fn drop(&mut self) {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_disconnect, self.ptr() as *mut _) }
    }
}

/// A guard over a read intention.
///
/// See `WlDisplay::prepare_read()` for details about its use.
pub struct ReadEventsGuard {
    display: *mut wl_display
}

impl ReadEventsGuard {
    /// Read events
    ///
    /// Reads events from the server socket. If other `ReadEventsGuard` exists, will block
    /// until they are all destroyed.
    pub fn read_events(self) -> io::Result<i32> {
        let ret = unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_read_events, self.display) };
        // Don't run destructor that would cancel the read intent
        ::std::mem::forget(self);
        if ret >= 0 { Ok(ret) } else { Err(io::Error::last_os_error()) }
    }

    /// Cancel the read
    ///
    /// Will cancel the read intention associated with this guard. Never blocks.
    ///
    /// Has the same effet as letting the guard go out of scope.
    pub fn cancel(self) {
        // just run the destructor
    }
}

impl Drop for ReadEventsGuard {
    fn drop(&mut self) {
        unsafe { ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_display_cancel_read, self.display) }
    }
}
