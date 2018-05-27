use std::cell::RefCell;
use std::io::{Error as IoError, Result as IoResult};
use std::os::raw::{c_int, c_void};
use std::os::unix::io::RawFd;
use std::rc::Rc;
use std::sync::atomic;
use std::sync::Arc;

use wayland_commons::{downcast_impl, Implementation};

use display::DisplayInner;
use sources::{FdEvent, FdInterest, IdleSource, SignalEvent, Source, TimerEvent};

#[cfg(feature = "native_lib")]
use wayland_sys::server::*;

pub(crate) struct EventLoopInner {
    #[cfg(feature = "native_lib")]
    wlevl: *mut wl_event_loop,
    pub(crate) inner: Option<Rc<DisplayInner>>,
}

/// An event loop
///
/// This is an event loop primitive provided by the wayland C libraries.
/// It is notably used for processing messages from the different clients
/// of your server, but additionnal event sources can be associated to it.
///
/// You can also create other event loops (for a multithreaded server for example),
/// however the wayland clients can only be processed from the original event loop
/// created at the same time as the display.
///
/// The event loops *cannot* be moved accross threads, so make sure you create them
/// on the thread you want to use them.
pub struct EventLoop {
    // EventLoop is *not* Send
    inner: Rc<EventLoopInner>,
    stop_signal: Arc<atomic::AtomicBool>,
}

/// An event loop token
///
/// This token allows some manipulations of the event loop, mainly
/// inserting new event sources in it.
///
/// These token are light and clone-able, allowing easy access to these
/// functions without needing to share access to the main `EventLoop` object.
#[derive(Clone)]
pub struct LoopToken {
    pub(crate) inner: Rc<EventLoopInner>,
}

/// An event loop signal
///
/// This handle can be cloned and be send accross threads, and allows you to
/// signal the event loop to stop running if you use the `EventLoop::run()`
/// method.
pub struct LoopSignal {
    inner: Arc<atomic::AtomicBool>,
}

impl LoopSignal {
    /// Signal the event loop to stop running
    pub fn stop(&self) {
        self.inner.store(true, atomic::Ordering::Release);
    }
}

impl EventLoop {
    /// Create a new event loop
    pub fn new() -> EventLoop {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let ptr = unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_create,) };
            EventLoop {
                inner: Rc::new(EventLoopInner {
                    wlevl: ptr,
                    inner: None,
                }),
                stop_signal: Arc::new(atomic::AtomicBool::new(false)),
            }
        }
    }

    #[cfg(feature = "native_lib")]
    pub(crate) unsafe fn display_new(disp_inner: Rc<DisplayInner>, ptr: *mut wl_event_loop) -> EventLoop {
        EventLoop {
            inner: Rc::new(EventLoopInner {
                wlevl: ptr,
                inner: Some(disp_inner),
            }),
            stop_signal: Arc::new(atomic::AtomicBool::new(false)),
        }
    }

    /// Retrieve a `LoopToken` associated to this event loop
    pub fn token(&self) -> LoopToken {
        LoopToken {
            inner: self.inner.clone(),
        }
    }

    /// Retrieve a `LoopSignal` associated to this event loop
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
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
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
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        loop {
            if let Some(ref display_inner) = self.inner.inner {
                unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_flush_clients, display_inner.ptr) };
            }
            self.dispatch(None)?;
            if self.stop_signal.load(atomic::Ordering::Acquire) {
                return Ok(());
            }
        }
    }
}

impl LoopToken {
    /// Add a File Descriptor event source to this event loop
    ///
    /// The interest in read/write capability for this FD must be provided
    /// (and can be changed afterwards using the returned object), and the
    /// associated implementation will be called whenever these capabilities are
    /// satisfied, during the dispatching of this event loop.
    pub fn add_fd_event_source<Impl>(
        &self,
        fd: RawFd,
        interest: FdInterest,
        implementation: Impl,
    ) -> Result<Source<FdEvent>, (IoError, Impl)>
    where
        Impl: Implementation<(), FdEvent> + 'static,
    {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let data = Box::new(Box::new(implementation) as Box<Implementation<(), FdEvent>>);
            let ret = unsafe {
                ffi_dispatch!(
                    WAYLAND_SERVER_HANDLE,
                    wl_event_loop_add_fd,
                    self.inner.wlevl,
                    fd,
                    interest.bits(),
                    ::sources::event_source_fd_dispatcher,
                    &*data as *const _ as *mut c_void
                )
            };
            if ret.is_null() {
                Err((
                    IoError::last_os_error(),
                    *(downcast_impl(*data).map_err(|_| ()).unwrap()),
                ))
            } else {
                Ok(Source::make(ret, data))
            }
        }
    }

    /// Add a timer event source to this event loop
    ///
    /// It is a countdown, which can be reset using the struct
    /// returned by this function. When the countdown reaches 0,
    /// the implementation is called in the dispatching of
    /// this event loop.
    pub fn add_timer_event_source<Impl>(
        &self,
        implementation: Impl,
    ) -> Result<Source<TimerEvent>, (IoError, Impl)>
    where
        Impl: Implementation<(), TimerEvent> + 'static,
    {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let data = Box::new(Box::new(implementation) as Box<Implementation<(), TimerEvent>>);
            let ret = unsafe {
                ffi_dispatch!(
                    WAYLAND_SERVER_HANDLE,
                    wl_event_loop_add_timer,
                    self.inner.wlevl,
                    ::sources::event_source_timer_dispatcher,
                    &*data as *const _ as *mut c_void
                )
            };
            if ret.is_null() {
                Err((
                    IoError::last_os_error(),
                    *(downcast_impl(*data).map_err(|_| ()).unwrap()),
                ))
            } else {
                Ok(Source::make(ret, data))
            }
        }
    }

    /// Add a signal event source to this event loop
    ///
    /// This will listen for a given unix signal (by setting up
    /// a signalfd for it) and call the implementation whenever
    /// the program receives this signal. Calls are made during the
    /// dispatching of this event loop.
    pub fn add_signal_event_source<Impl>(
        &self,
        signal: ::nix::sys::signal::Signal,
        implementation: Impl,
    ) -> Result<Source<SignalEvent>, (IoError, Impl)>
    where
        Impl: Implementation<(), SignalEvent> + 'static,
    {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let data = Box::new(Box::new(implementation) as Box<Implementation<(), SignalEvent>>);
            let ret = unsafe {
                ffi_dispatch!(
                    WAYLAND_SERVER_HANDLE,
                    wl_event_loop_add_signal,
                    self.inner.wlevl,
                    signal as c_int,
                    ::sources::event_source_signal_dispatcher,
                    &*data as *const _ as *mut c_void
                )
            };
            if ret.is_null() {
                Err((
                    IoError::last_os_error(),
                    *(downcast_impl(*data).map_err(|_| ()).unwrap()),
                ))
            } else {
                Ok(Source::make(ret, data))
            }
        }
    }

    /// Add an idle event source to this event loop
    ///
    /// This is a kind of "defer this computation for when there is nothing else to do".
    ///
    /// The provided implementation callback will be called when the event loop has finished
    /// processing all the pending I/O. This callback will be fired exactly once the first
    /// time this condition is met.
    ///
    /// You can cancel or retrieve the implementation after it has fired using the
    /// returned `IdleEventSource`.
    pub fn add_idle_event_source<Impl>(&self, implementation: Impl) -> IdleSource
    where
        Impl: Implementation<(), ()> + 'static,
    {
        #[cfg(not(feature = "native_lib"))]
        {
            unimplemented!()
        }
        #[cfg(feature = "native_lib")]
        {
            let data = Rc::new(RefCell::new((
                Box::new(implementation) as Box<Implementation<(), ()>>,
                false,
            )));
            let ret = unsafe {
                ffi_dispatch!(
                    WAYLAND_SERVER_HANDLE,
                    wl_event_loop_add_idle,
                    self.inner.wlevl,
                    ::sources::event_source_idle_dispatcher,
                    Rc::into_raw(data.clone()) as *mut _
                )
            };
            IdleSource::make(ret, data)
        }
    }

    /// Checks whether given resource is indeed linked to this
    /// event loop
    #[cfg(feature = "native_lib")]
    pub(crate) unsafe fn matches(&self, resource_ptr: *mut wl_resource) -> bool {
        let client_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, resource_ptr);
        let display_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_get_display, client_ptr);
        let event_loop_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, display_ptr);
        return event_loop_ptr == self.inner.wlevl;
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
