use super::{FromOpt, ShmPool};

use ffi::interfaces::shm_pool::wl_shm_pool_create_buffer;
use ffi::interfaces::buffer::{wl_buffer, wl_buffer_destroy};
use ffi::FFI;

/// A view into a memory pool.
///
/// A buffer represents a given view into a memory pool. They only
/// serve to notify the wayland server about how the contents of the
/// memory pool must be read. To actually modify the data, you need
/// to directly access the object you created the memory pool from.
pub struct Buffer<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_buffer
}

impl<'a, 'b> FromOpt<(&'b ShmPool<'a>, i32, i32, i32, i32, u32)> for Buffer<'a> {
    fn from((pool, offset, width, height, stride, format): (&ShmPool<'a>, i32, i32, i32, i32, u32))
            -> Option<Buffer<'a>> {
        let ptr = unsafe { wl_shm_pool_create_buffer(
            pool.ptr_mut(), offset, width, height, stride, format) };
        if ptr.is_null() {
            None
        } else {
            Some(Buffer {
                _t: ::std::marker::PhantomData,
                ptr: ptr
            })
        }
    }
}

impl<'a> Drop for Buffer<'a> {
    fn drop(&mut self) {
        unsafe { wl_buffer_destroy(self.ptr) };
    }
}


impl<'a> FFI for Buffer<'a> {
    type Ptr = wl_buffer;

    fn ptr(&self) -> *const wl_buffer {
        self.ptr as *const wl_buffer
    }

    unsafe fn ptr_mut(&self) -> *mut wl_buffer {
        self.ptr
    }
}