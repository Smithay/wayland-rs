use super::{From, FromOpt, Buffer, Shm, ShmFormat};

use ffi::interfaces::shm::wl_shm_create_pool;
use ffi::interfaces::shm_pool::{wl_shm_pool, wl_shm_pool_destroy};
use ffi::FFI;

pub struct ShmPool<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_shm_pool
}

impl<'a> ShmPool<'a> {
    pub fn create_buffer(&self, offset: i32, width: i32, height: i32, stride: i32, format: ShmFormat)
            -> Option<Buffer<'a>> {
        FromOpt::from((self, offset, width, height, stride, format as u32))
    }
}

impl<'a, 'b> From<(&'a Shm<'b>, i32, i32)> for ShmPool<'a> {
    fn from((shm, fd, size): (&'a Shm<'b>, i32, i32)) -> ShmPool<'a> {
        let ptr = unsafe { wl_shm_create_pool(shm.ptr_mut(), fd, size) };
        ShmPool {
            _t: ::std::marker::PhantomData,
            ptr: ptr
        }
    }
}

impl<'a> Drop for ShmPool<'a> {
    fn drop(&mut self) {
        unsafe { wl_shm_pool_destroy(self.ptr) };
    }
}

impl<'a> FFI<wl_shm_pool> for ShmPool<'a> {
    fn ptr(&self) -> *const wl_shm_pool {
        self.ptr as *const wl_shm_pool
    }

    unsafe fn ptr_mut(&self) -> *mut wl_shm_pool {
        self.ptr
    }
}

