use std::any::Any;
use std::io::Result as IoResult;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_void, c_int};

use wayland_sys::client::*;
use wayland_sys::common::*;
use {Handler, Proxy};

pub struct EventQueueHandle {
    handlers: Vec<Box<Any>>
}

pub struct StateGuard<'evq> {
    evq: &'evq mut EventQueue
}

pub struct EventQueue {
    display: *mut wl_display,
    wlevq: Option<*mut wl_event_queue>,
    handle: Box<EventQueueHandle>
}

impl EventQueue {
    pub fn fetch(&mut self) -> IoResult<usize> {
        unimplemented!()
    }
    
    pub fn dispatch(&mut self) -> i32 {
        match self.wlevq {
            Some(evq) => unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_dispatch_queue_pending,
                    self.display,
                    evq
                )
            },
            None => unsafe {
                ffi_dispatch!(
                    WAYLAND_CLIENT_HANDLE,
                    wl_display_dispatch_pending,
                    self.display
                )
            }
        }
    }
    
    pub fn state(&mut self) -> StateGuard {
        StateGuard { evq: self }
    }
}

impl Deref for EventQueue {
    type Target = EventQueueHandle;
    fn deref(&self) -> &EventQueueHandle {
        &*self.handle
    }
}

impl DerefMut for EventQueue {
    fn deref_mut(&mut self) -> &mut EventQueueHandle {
        &mut *self.handle
    }
}

unsafe extern "C" fn dispatch_func<P: Proxy, H: Handler<P>>(
    handler: *const c_void,
    proxy: *mut c_void,
    opcode: u32,
    _msg: *const wl_message,
    args: *const wl_argument
) -> c_int {
    // This cast from *const to *mut is legit because we enforce that a Handler
    // can only be assigned to a single EventQueue.
    // (this is actually the whole point of the design of this lib)
    let handler = &mut *(handler as *const H as *mut H);
    let proxy = P::from_ptr(proxy as *mut wl_proxy);
    let evqhandle = &mut *(ffi_dispatch!(
        WAYLAND_CLIENT_HANDLE, wl_proxy_get_user_data, proxy.ptr()
    ) as *mut EventQueueHandle);
    // This is VERY bad: if Handler::message panics (which it can easily, as it calls user-provided
    // code), we unwind all the way into the C code, which is completely UB.
    // However, we have no way to report failure to the caller C lib (it actually ignores our
    // return value) so catch_unwind is not an option.
    // So... ¯\_(ツ)_/¯
    let ret = handler.message(evqhandle, &proxy, opcode, args);
    match ret {
        Ok(()) => 0,   // all went well
        Err(()) => {
            // an unknown opcode was dispatched, this is not normal (but we cannot panic)
            let _ = write!(::std::io::stderr(), "[wayland-client error] Attempted to dispatch an unknown opcode");
            -1
        }
    }
}
