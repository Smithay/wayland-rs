

use EventLoopHandle;
use std::io::Error as IoError;
use std::io::Write;
use std::os::raw::{c_int, c_void};
use std::os::unix::io::RawFd;
use wayland_sys::server::*;

/// fd_event_source
///
/// A handle to a registered FD event source
///
/// Dropping this struct does not remove the event source,
/// use the `remove` method for that.
pub struct FdEventSource<ID> {
    ptr: *mut wl_event_source,
    data: *mut (FdEventSourceImpl<ID>, *mut EventLoopHandle, ID),
}

bitflags!{
    /// Flags to register interest on a file descriptor
    pub struct FdInterest: u32 {
        /// Interest to be notified when the file descriptor is readable
        const READ  = 0x01;
        /// Interest to be notified when the file descriptor is writable
        const WRITE = 0x02;
    }
}

pub fn make_fd_event_source<ID: 'static>(ptr: *mut wl_event_source,
                                         data: Box<(FdEventSourceImpl<ID>, *mut EventLoopHandle, ID)>)
                                         -> FdEventSource<ID> {
    FdEventSource {
        ptr: ptr,
        data: Box::into_raw(data),
    }
}

impl<ID> FdEventSource<ID> {
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
    ///
    /// Returns the implementation data in case you have something to do with it.
    pub fn remove(self) -> ID {
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_source_remove, self.ptr);
            let data = Box::from_raw(self.data);
            data.2
        }
    }
}

/// Implementation for FD events
pub struct FdEventSourceImpl<ID> {
    /// The FD is ready to be read/written
    ///
    /// Details of the capability state are given as argument.
    pub ready: fn(evlh: &mut EventLoopHandle, idata: &mut ID, fd: RawFd, mask: FdInterest),
    /// An error occured with this FD
    ///
    /// Most likely it won't be usable any longer
    pub error: fn(evlh: &mut EventLoopHandle, idata: &mut ID, fd: RawFd, mask: IoError),
}


pub unsafe extern "C" fn event_source_fd_dispatcher<ID>(fd: c_int, mask: u32, data: *mut c_void) -> c_int
where
    ID: 'static,
{
    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let data = &mut *(data as *mut (FdEventSourceImpl<ID>, *mut EventLoopHandle, ID));
        let implem = &data.0;
        let evlh = &mut *(data.1);
        let idata = &mut data.2;
        if mask & 0x08 > 0 {
            // EPOLLERR
            use nix::sys::socket;
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
            (implem.error)(evlh, idata, fd, IoError::from_raw_os_error(err));
        } else if mask & 0x04 > 0 {
            // EPOLLHUP
            (implem.error)(
                evlh,
                idata,
                fd,
                IoError::new(::std::io::ErrorKind::ConnectionAborted, ""),
            )
        } else {
            let mut bits = FdInterest::empty();
            if mask & 0x02 > 0 {
                bits = bits | FdInterest::WRITE;
            }
            if mask & 0x01 > 0 {
                bits = bits | FdInterest::READ;
            }
            (implem.ready)(evlh, idata, fd, bits)
        }
    });
    match ret {
        Ok(()) => return 0, // all went well
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

/// timer_event_source
///
/// A handle to a registered timer event source
///
/// Dropping this struct does not remove the event source,
/// use the `remove` method for that.
pub struct TimerEventSource<ID> {
    ptr: *mut wl_event_source,
    data: *mut (TimerEventSourceImpl<ID>, *mut EventLoopHandle, ID),
}


pub fn make_timer_event_source<ID>(ptr: *mut wl_event_source,
                                   data: Box<(TimerEventSourceImpl<ID>, *mut EventLoopHandle, ID)>)
                                   -> TimerEventSource<ID> {
    TimerEventSource {
        ptr: ptr,
        data: Box::into_raw(data),
    }
}

impl<ID> TimerEventSource<ID> {
    /// Set the delay of this timer
    ///
    /// The callback will be called during the next dispatch of the
    /// event loop after this time (in milliseconds) is elapsed.
    ///
    /// Manually the delay to 0 stops the timer (the callback won't be
    /// called).
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
    ///
    /// Returns the implementation data in case you have something to do with it.
    pub fn remove(self) -> ID {
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_source_remove, self.ptr);
            let data = Box::from_raw(self.data);
            data.2
        }
    }
}

/// Called when the countdown reaches 0
pub type TimerEventSourceImpl<ID> = fn(&mut EventLoopHandle, idata: &mut ID);

pub unsafe extern "C" fn event_source_timer_dispatcher<ID>(data: *mut c_void) -> c_int
where
    ID: 'static,
{
    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let data = &mut *(data as *mut (TimerEventSourceImpl<ID>, *mut EventLoopHandle, ID));
        let cb = data.0;
        let evlh = &mut *(data.1);
        let idata = &mut data.2;
        cb(evlh, idata);
    });
    match ret {
        Ok(()) => return 0, // all went well
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

/// signal_event_source
///
/// A handle to a registered signal event source
///
/// Dropping this struct does not remove the event source,
/// use the `remove` method for that.
pub struct SignalEventSource<ID> {
    ptr: *mut wl_event_source,
    data: *mut (SignalEventSourceImpl<ID>, *mut EventLoopHandle, ID),
}


pub fn make_signal_event_source<ID: 'static>(ptr: *mut wl_event_source,
                                             data: Box<
    (SignalEventSourceImpl<ID>, *mut EventLoopHandle, ID),
>)
                                             -> SignalEventSource<ID> {
    SignalEventSource {
        ptr: ptr,
        data: Box::into_raw(data),
    }
}

impl<ID> SignalEventSource<ID> {
    /// Remove this event source from its event loop
    ///
    /// Returns the implementation data in case you have something to do with it.
    pub fn remove(self) -> ID {
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_source_remove, self.ptr);
            let data = Box::from_raw(self.data);
            data.2
        }
    }
}

/// A signal has been received
///
/// The signal number is given has argument
pub type SignalEventSourceImpl<ID> = fn(
                                      &mut EventLoopHandle,
                                      idata: &mut ID,
                                      signal: ::nix::sys::signal::Signal,
);

pub unsafe extern "C" fn event_source_signal_dispatcher<ID>(signal: c_int, data: *mut c_void) -> c_int
where
    ID: 'static,
{
    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let data = &mut *(data as *mut (SignalEventSourceImpl<ID>, *mut EventLoopHandle, ID));
        let cb = data.0;
        let evlh = &mut *(data.1);
        let idata = &mut data.2;
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
        cb(evlh, idata, sig);
    });
    match ret {
        Ok(()) => return 0, // all went well
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
