use std::cell::RefCell;
use std::io::{Error as IoError, Result as IoResult};
use std::os::raw::{c_int, c_void};
use std::os::unix::io::RawFd;
use std::rc::Rc;

use wayland_sys::server::*;

use super::{DisplayInner, IdleSourceInner, SourceInner};

use sources::*;

pub(crate) struct EventLoopInner {
    wlevl: *mut wl_event_loop,
    pub(crate) display: Option<Rc<RefCell<DisplayInner>>>,
}

impl Drop for EventLoopInner {
    fn drop(&mut self) {
        if self.display.is_none() {
            // only destroy the event_loop if it's not the one from the display
            unsafe {
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_destroy, self.wlevl);
            }
        }
    }
}

impl EventLoopInner {
    pub(crate) fn display_new(display: Rc<RefCell<DisplayInner>>, ptr: *mut wl_event_loop) -> EventLoopInner {
        EventLoopInner {
            display: Some(display),
            wlevl: ptr,
        }
    }

    pub(crate) fn new() -> EventLoopInner {
        let ptr = unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_create,) };
        EventLoopInner {
            wlevl: ptr,
            display: None,
        }
    }

    pub fn dispatch(&self, timeout: Option<u32>) -> IoResult<u32> {
        use std::i32;
        let timeout = match timeout {
            None => -1,
            Some(v) if v >= (i32::MAX as u32) => i32::MAX,
            Some(v) => (v as i32),
        };
        let ret =
            unsafe { ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_dispatch, self.wlevl, timeout) };
        if ret >= 0 {
            Ok(ret as u32)
        } else {
            Err(IoError::last_os_error())
        }
    }

    pub(crate) fn flush_clients_if_display(&self) {
        if let Some(ref display) = self.display {
            display.borrow_mut().flush_clients();
        }
    }

    pub fn add_fd_event_source<F>(
        &self,
        fd: RawFd,
        interest: FdInterest,
        implementation: F,
    ) -> IoResult<SourceInner<FdEvent>>
    where
        F: FnMut(FdEvent) + 'static,
    {
        let data = Box::new(Box::new(implementation) as Box<FnMut(FdEvent)>);
        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_fd,
                self.wlevl,
                fd,
                interest.bits(),
                super::source::event_source_fd_dispatcher,
                &*data as *const _ as *mut c_void
            )
        };
        if ret.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(SourceInner::make(ret, data))
        }
    }

    pub fn add_timer_event_source<F>(&self, implementation: F) -> IoResult<SourceInner<TimerEvent>>
    where
        F: FnMut(TimerEvent) + 'static,
    {
        let data = Box::new(Box::new(implementation) as Box<FnMut(TimerEvent)>);
        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_timer,
                self.wlevl,
                super::source::event_source_timer_dispatcher,
                &*data as *const _ as *mut c_void
            )
        };
        if ret.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(SourceInner::make(ret, data))
        }
    }

    pub fn add_signal_event_source<F>(
        &self,
        signal: ::nix::sys::signal::Signal,
        implementation: F,
    ) -> IoResult<SourceInner<SignalEvent>>
    where
        F: FnMut(SignalEvent) + 'static,
    {
        let data = Box::new(Box::new(implementation) as Box<FnMut(SignalEvent)>);
        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_signal,
                self.wlevl,
                signal as c_int,
                super::source::event_source_signal_dispatcher,
                &*data as *const _ as *mut c_void
            )
        };
        if ret.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(SourceInner::make(ret, data))
        }
    }

    pub fn add_idle_event_source<F>(&self, implementation: F) -> IdleSourceInner
    where
        F: FnMut() + 'static,
    {
        let data = Rc::new(RefCell::new((Box::new(implementation) as Box<FnMut()>, false)));
        let ret = unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_loop_add_idle,
                self.wlevl,
                super::source::event_source_idle_dispatcher,
                Rc::into_raw(data.clone()) as *mut _
            )
        };
        IdleSourceInner::make(ret, data)
    }

    pub(crate) unsafe fn matches(&self, resource_ptr: *mut wl_resource) -> bool {
        let client_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, resource_ptr);
        let display_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_client_get_display, client_ptr);
        let event_loop_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, display_ptr);
        return event_loop_ptr == self.wlevl;
    }
}
