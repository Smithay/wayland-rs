use std::os::unix::io::AsRawFd;

use super::{From, ShmPool};

use ffi::interfaces::shm::{wl_shm, wl_shm_destroy};
use ffi::{FFI, Bind, abi};

pub use ffi::enums::wl_shm_format as ShmFormat;

/// The shared memory controller.
///
/// This object can be queried for memory_pools used for the buffers.
pub struct Shm<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_shm
}

impl<'a> Shm<'a> {
    /// Creates a shared memory pool from given file descriptor.
    ///
    /// The server will internally `mmap` the sepcified `size` number of bytes from
    /// this file descriptor.
    /// The created ShmPool will have access to these bytes exactly.
    pub fn pool_from_fd<'b, F: AsRawFd>(&'b self, fd: &F, size: i32) -> ShmPool<'b> {
        From::from((self, fd.as_raw_fd(), size))
    }
}

impl<'a, R> Bind<'a, R> for Shm<'a> {
    fn interface() -> &'static abi::wl_interface {
        abi::WAYLAND_CLIENT_HANDLE.wl_shm_interface
    }

    unsafe fn wrap(ptr: *mut wl_shm, _parent: &'a R) -> Shm<'a> {
        Shm {
            _t: ::std::marker::PhantomData,
            ptr: ptr
        }
    }
}

impl<'a> Drop for Shm<'a> {
    fn drop(&mut self) {
        unsafe { wl_shm_destroy(self.ptr_mut()) };
    }
}

impl<'a> FFI for Shm<'a> {
    type Ptr = wl_shm;
   fn ptr(&self) -> *const wl_shm {
        self.ptr as *const wl_shm
    }

    unsafe fn ptr_mut(&self) -> *mut wl_shm {
        self.ptr
    }
}
