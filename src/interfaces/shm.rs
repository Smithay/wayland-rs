use std::os::unix::io::AsRawFd;

use super::{From, Registry, ShmPool};

use ffi::interfaces::shm::{wl_shm, wl_shm_destroy};
use ffi::interfaces::registry::wl_registry_bind;
use ffi::{FFI, abi};

pub use ffi::enums::wl_shm_format as ShmFormat;

/// The shared memory controller.
///
/// This object can be queried for memory_pools used forthe buffers.
pub struct Shm<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_shm
}

impl<'a> Shm<'a> {
    pub fn pool_from_fd<'b, F: AsRawFd>(&'b self, fd: &F, size: i32) -> ShmPool<'b> {
        From::from((self, fd.as_raw_fd(), size))
    }
}

impl<'a, 'b> From<(&'a Registry<'b>, u32, u32)> for Shm<'a> {
    fn from((registry, id, version): (&'a Registry, u32, u32)) -> Shm<'a> {
        let ptr = unsafe { wl_registry_bind(
            registry.ptr_mut(),
            id,
            &abi::wl_shm_interface,
            version
        ) as *mut wl_shm };

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

impl<'a> FFI<wl_shm> for Shm<'a> {
    fn ptr(&self) -> *const wl_shm {
        self.ptr as *const wl_shm
    }

    unsafe fn ptr_mut(&self) -> *mut wl_shm {
        self.ptr
    }
}
