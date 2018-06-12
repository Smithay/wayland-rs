use std::cell::RefCell;
use std::io::Error as IoError;
use std::os::raw::{c_int, c_void};
use std::rc::Rc;

use wayland_sys::server::*;

use Implementation;

use sources::*;

pub(crate) struct SourceInner<E> {
    _e: ::std::marker::PhantomData<*const E>,
    ptr: *mut wl_event_source,
    data: *mut Box<Implementation<(), E>>,
}

impl<E> SourceInner<E> {
    pub(crate) fn make(ptr: *mut wl_event_source, data: Box<Box<Implementation<(), E>>>) -> SourceInner<E> {
        SourceInner {
            _e: ::std::marker::PhantomData,
            ptr: ptr,
            data: Box::into_raw(data),
        }
    }

    pub(crate) fn remove(self) -> Box<Implementation<(), E>> {
        unsafe {
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_source_remove, self.ptr);
            let data = Box::from_raw(self.data);
            *data
        }
    }
}

// FD event source

impl SourceInner<FdEvent> {
    pub(crate) fn update_mask(&mut self, mask: FdInterest) {
        unsafe {
            ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_event_source_fd_update,
                self.ptr,
                mask.bits()
            );
        }
    }
}

pub(crate) unsafe extern "C" fn event_source_fd_dispatcher(fd: c_int, mask: u32, data: *mut c_void) -> c_int {
    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let implem = &mut *(data as *mut Box<Implementation<(), FdEvent>>);
        if mask & 0x08 > 0 {
            // EPOLLERR
            use nix::sys::socket;
            let err = match socket::getsockopt(fd, socket::sockopt::SocketError) {
                Ok(err) => err,
                Err(_) => {
                    // error while retrieving the error code ???
                    eprintln!(
                        "[wayland-server error] Error while retrieving error code on socket {}, aborting.",
                        fd
                    );
                    ::libc::abort();
                }
            };
            implem.receive(
                FdEvent::Error {
                    fd: fd,
                    error: IoError::from_raw_os_error(err),
                },
                (),
            );
        } else if mask & 0x04 > 0 {
            // EPOLLHUP
            implem.receive(
                FdEvent::Error {
                    fd: fd,
                    error: IoError::new(::std::io::ErrorKind::ConnectionAborted, ""),
                },
                (),
            );
        } else {
            let mut bits = FdInterest::empty();
            if mask & 0x02 > 0 {
                bits = bits | FdInterest::WRITE;
            }
            if mask & 0x01 > 0 {
                bits = bits | FdInterest::READ;
            }
            implem.receive(FdEvent::Ready { fd: fd, mask: bits }, ());
        }
    });
    match ret {
        Ok(()) => return 0, // all went well
        Err(_) => {
            // a panic occured
            eprintln!(
                "[wayland-server error] A handler for fd {} event source panicked, aborting.",
                fd
            );
            ::libc::abort();
        }
    }
}

// Timer event source

impl SourceInner<TimerEvent> {
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
}

pub(crate) unsafe extern "C" fn event_source_timer_dispatcher(data: *mut c_void) -> c_int {
    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let implem = &mut *(data as *mut Box<Implementation<(), TimerEvent>>);
        implem.receive(TimerEvent, ());
    });
    match ret {
        Ok(()) => return 0, // all went well
        Err(_) => {
            // a panic occured
            eprintln!("[wayland-server error] A handler for a timer event source panicked, aborting.",);
            ::libc::abort();
        }
    }
}

// Signal event source

pub(crate) unsafe extern "C" fn event_source_signal_dispatcher(signal: c_int, data: *mut c_void) -> c_int {
    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let implem = &mut *(data as *mut Box<Implementation<(), SignalEvent>>);
        let sig = match ::nix::sys::signal::Signal::from_c_int(signal) {
            Ok(sig) => sig,
            Err(_) => {
                // Actually, this cannot happen, as we cannot register an event source for
                // an unknown signal...
                eprintln!(
                    "[wayland-server error] Unknown signal in signal event source: {}, aborting.",
                    signal
                );
                ::libc::abort();
            }
        };
        implem.receive(SignalEvent(sig), ());
    });
    match ret {
        Ok(()) => return 0, // all went well
        Err(_) => {
            // a panic occured
            eprintln!("[wayland-server error] A handler for a timer event source panicked, aborting.",);
            ::libc::abort();
        }
    }
}

// Idle event source

pub(crate) struct IdleSourceInner {
    ptr: *mut wl_event_source,
    data: Rc<RefCell<(Box<Implementation<(), ()>>, bool)>>,
}

impl IdleSourceInner {
    pub(crate) fn make(
        ptr: *mut wl_event_source,
        data: Rc<RefCell<(Box<Implementation<(), ()>>, bool)>>,
    ) -> IdleSourceInner {
        IdleSourceInner { ptr, data }
    }

    pub(crate) fn remove(self) -> Box<Implementation<(), ()>> {
        let dispatched = self.data.borrow().1;
        if !dispatched {
            unsafe {
                // unregister this event source
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_source_remove, self.ptr);
                // recreate the outstanding reference that was not consumed
                let _ = Rc::from_raw(&*self.data);
            }
        }
        // we are now the only oustanding reference
        let data = Rc::try_unwrap(self.data)
            .unwrap_or_else(|_| panic!("Idle Rc was not singly owned."))
            .into_inner();
        data.0
    }
}

pub(crate) unsafe extern "C" fn event_source_idle_dispatcher(data: *mut c_void) {
    // We don't need to worry about panic-safeness, because if there is a panic,
    // we'll abort the process, so no access to corrupted data is possible.
    let ret = ::std::panic::catch_unwind(move || {
        let data = &*(data as *mut RefCell<(Box<Implementation<(), ()>>, bool)>);
        let mut data = data.borrow_mut();
        data.0.receive((), ());
    });
    match ret {
        Ok(()) => {
            // all went well
            // free the refence to the idata, as this event source cannot be called again
            let data = Rc::from_raw(data as *mut RefCell<(Box<Implementation<(), ()>>, bool)>);
            // store that the dispatching occured
            data.borrow_mut().1 = true;
        }
        Err(_) => {
            // a panic occured
            eprintln!("[wayland-server error] A handler for a timer event source panicked, aborting.",);
            ::libc::abort();
        }
    }
}
