use wayland_sys::server::*;

use std::os::unix::io::RawFd;
use std::os::raw::{c_int,c_void};

use std::io::Error as IoError;
use std::io::Write;

/*
 * fd_event_source
 */

/// A handle to a registered FD event source
///
/// Dropping this struct does not remove the event source,
/// use the `remove` method for that.
pub struct FdEventSource {
    ptr: *mut wl_event_source
}

bitflags!{
    /// Flags to register interest on a file descriptor
    pub flags FdInterest: u32 {
        /// Interest to be notified when the file descriptor is readable
        const READ  = 0x01,
        /// Interest to be notified when the file descriptor is writable
        const WRITE = 0x02
    }
}

pub fn make_fd_event_source(ptr: *mut wl_event_source) -> FdEventSource {
    FdEventSource {
        ptr: ptr
    }
}

impl FdEventSource {
    /// Change the registered interest for this FD
    pub fn update_mask(&mut self, mask: FdInterest) {
        unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_source_fd_update,
                self.ptr,
                mask.bits()
            );
        }
    }

    /// Remove this event source from its event loop
    pub fn remove(self) {
        unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_source_remove,
                self.ptr
            );
        }
    }
}

/// Trait for handlers for FD events
pub trait FdEventSourceHandler {
    /// The FD is ready to be read/written
    ///
    /// Details of the capability state are given as argument.
    fn ready(&mut self, fd: RawFd, mask: FdInterest);
    /// An error occured with this FD
    ///
    /// Most likely it won't be usable any longer
    fn error(&mut self, fd: RawFd, error: IoError);
}

pub unsafe extern "C" fn event_source_fd_dispatcher<H>(fd: c_int, mask: u32, data: *mut c_void) -> c_int
    where H: FdEventSourceHandler
{
// We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let handler = &mut *(data as *mut H);
        if mask & 0x04 > 0 {
            // EPOLLERR
            use ::nix::sys::socket;
            let err = match socket::getsockopt(fd, socket::sockopt::SocketError) {
                Ok(err) => err,
                Err(_) => {
                    // error while retrieving the error code ???
                    let _ = write!(
                        ::std::io::stderr(),
                        "[wayland-server error] Error while retrieving error code on socket {}, aborting.",
                        fd
                    );
                    ::libc::abort();
                }
            };
            handler.error(fd, IoError::from_raw_os_error(err));
        } else if mask & 0x03 > 0 {
            // EPOLLHUP
            handler.error(fd, IoError::new(::std::io::ErrorKind::ConnectionAborted, ""))
        } else {
            let mut bits = FdInterest::empty();
            if mask & 0x02 > 0 { bits = bits | WRITE; }
            if mask & 0x01 > 0 { bits = bits | READ; }
            handler.ready(fd, bits)
        }
    });
    match ret {
        Ok(()) => return 0,   // all went well
        Err(_) => {
            // a panic occured
            let _ = write!(
                ::std::io::stderr(),
                "[wayland-server error] A handler for fd {} event source panicked, aborting.",
                fd
            );
            ::libc::abort();
        }
    }
}

/*
 * timer_event_source
 */

/// A handle to a registered timer event source
///
/// Dropping this struct does not remove the event source,
/// use the `remove` method for that.
pub struct TimerEventSource {
    ptr: *mut wl_event_source
}


pub fn make_timer_event_source(ptr: *mut wl_event_source) -> TimerEventSource {
    TimerEventSource {
        ptr: ptr
    }
}

impl TimerEventSource {
    /// Set the delay of this timer
    ///
    /// The handler will be called during the next dispatch of the
    /// event loop after this time (in milliseconds) is elapsed.
    pub fn set_delay_ms(&mut self, delay: i32) {
        unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_source_timer_update,
                self.ptr,
                delay
            );
        }
    }

    /// Remove this event source from its event loop
    pub fn remove(self) {
        unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_source_remove,
                self.ptr
            );
        }
    }
}

/// Trait for handlers for timer event sources
pub trait TimerEventSourceHandler {
    /// The countdown has reached zero
    fn timeout(&mut self);
}

pub unsafe extern "C" fn event_source_timer_dispatcher<H>(data: *mut c_void) -> c_int
    where H: TimerEventSourceHandler
{
// We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let handler = &mut *(data as *mut H);
        handler.timeout();
    });
    match ret {
        Ok(()) => return 0,   // all went well
        Err(_) => {
            // a panic occured
            let _ = write!(
                ::std::io::stderr(),
                "[wayland-server error] A handler for a timer event source panicked, aborting.",
            );
            ::libc::abort();
        }
    }
}

/*
 * signal_event_source
 */

/// A handle to a registered signal event source
///
/// Dropping this struct does not remove the event source,
/// use the `remove` method for that.
pub struct SignalEventSource {
    ptr: *mut wl_event_source
}


pub fn make_signal_event_source(ptr: *mut wl_event_source) -> SignalEventSource {
    SignalEventSource {
        ptr: ptr
    }
}

impl SignalEventSource {
    /// Remove this event source from its event loop
    pub fn remove(self) {
        unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_source_remove,
                self.ptr
            );
        }
    }
}

/// Trait for handlers of signal event sources
pub trait SignalEventSourceHandler {
    /// A signal has been received
    ///
    /// The signal number is given has argument
    fn signal(&mut self, ::nix::sys::signal::Signal);
}

pub unsafe extern "C" fn event_source_signal_dispatcher<H>(signal: c_int, data: *mut c_void) -> c_int
    where H: SignalEventSourceHandler
{
// We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let handler = &mut *(data as *mut H);
        let sig = match ::nix::sys::signal::Signal::from_c_int(signal) {
            Ok(sig) => sig,
            Err(_) => {
                // Actually, this cannot happen, as we cannot register an event source for
                // an unknown signal...
                let _ = write!(
                    ::std::io::stderr(),
                    "[wayland-server error] Unknown signal in signal event source: {}, aborting.",
                    signal
                );
                ::libc::abort();
            }
        };
        handler.signal(sig);
    });
    match ret {
        Ok(()) => return 0,   // all went well
        Err(_) => {
            // a panic occured
            let _ = write!(
                ::std::io::stderr(),
                "[wayland-server error] A handler for a timer event source panicked, aborting.",
            );
            ::libc::abort();
        }
    }
}
