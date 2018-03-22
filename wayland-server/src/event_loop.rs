use std::io::{Error as IoError, Result as IoResult};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic;

use display::DisplayInner;

#[cfg(feature = "native_lib")]
use wayland_sys::server::*;

pub(crate) struct EventLoopInner {
    #[cfg(feature = "native_lib")] wlevl: *mut wl_event_loop,
    pub(crate) inner: Option<Arc<DisplayInner>>,
}

pub struct EventLoop {
    // EventLoop is *not* Send
    inner: Rc<EventLoopInner>,
    stop_signal: Arc<atomic::AtomicBool>,
}

pub struct LoopToken {
    pub(crate) inner: Rc<EventLoopInner>,
}

pub struct LoopSignal {
    inner: Arc<atomic::AtomicBool>,
}

impl LoopSignal {
    pub fn stop(&self) {
        self.inner.store(true, atomic::Ordering::Release);
    }
}

impl EventLoop {
    #[cfg(feature = "native_lib")]
    pub(crate) unsafe fn display_new(disp_inner: Arc<DisplayInner>, ptr: *mut wl_event_loop) -> EventLoop {
        EventLoop {
            inner: Rc::new(EventLoopInner {
                wlevl: ptr,
                inner: Some(disp_inner),
            }),
            stop_signal: Arc::new(atomic::AtomicBool::new(false)),
        }
    }

    pub fn token(&self) -> LoopToken {
        LoopToken {
            inner: self.inner.clone(),
        }
    }

    pub fn signal(&self) -> LoopSignal {
        LoopSignal {
            inner: self.stop_signal.clone(),
        }
    }

    /// Dispatch pending requests to their respective handlers
    ///
    /// If no request is pending, will block at most `timeout` ms if specified,
    /// or indefinitely if `timeout` is `None`.
    ///
    /// Returns the number of requests dispatched or an error.
    pub fn dispatch(&mut self, timeout: Option<u32>) -> IoResult<u32> {
        #[cfg(feature = "native_lib")]
        {
            use std::i32;
            let timeout = match timeout {
                None => -1,
                Some(v) if v >= (i32::MAX as u32) => i32::MAX,
                Some(v) => (v as i32),
            };
            let ret = unsafe {
                ffi_dispatch!(
                    WAYLAND_SERVER_HANDLE,
                    wl_event_loop_dispatch,
                    self.inner.wlevl,
                    timeout
                )
            };
            if ret >= 0 {
                Ok(ret as u32)
            } else {
                Err(IoError::last_os_error())
            }
        }
    }

    /// Runs the event loop
    ///
    /// This method will call repetitively the dispatch method,
    /// until one of the handlers call the `stop` method of an associated
    /// `LoopSignal`.
    ///
    /// If this event loop is attached to a display, it will also
    /// flush the events to the clients between two calls to
    /// `dispatch()`.
    ///
    /// Note that this method will block indefinitely on waiting events,
    /// as such, if you need to avoid a complete block even if no events
    /// are received, you should use the `dispatch()` method instead and
    /// set a timeout.
    pub fn run(&mut self) -> IoResult<()> {
        self.stop_signal.store(false, atomic::Ordering::Release);
        loop {
            if let Some(ref display_inner) = self.inner.inner {
                unsafe {
                    ffi_dispatch!(
                        WAYLAND_SERVER_HANDLE,
                        wl_display_flush_clients,
                        display_inner.ptr
                    )
                };
            }
            self.dispatch(None)?;
            if self.stop_signal.load(atomic::Ordering::Acquire) {
                return Ok(());
            }
        }
    }
}

impl Drop for EventLoopInner {
    fn drop(&mut self) {
        #[cfg(feature = "native_lib")]
        {
            if self.inner.is_none() {
                // only destroy the event_loop if it's not the one from the display
                unsafe {
                    ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_destroy, self.wlevl);
                }
            }
        }
    }
}
