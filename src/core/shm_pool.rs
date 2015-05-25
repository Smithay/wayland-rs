use super::{From, FromOpt, Buffer, Shm, ShmFormat};

use ffi::interfaces::shm::wl_shm_create_pool;
use ffi::interfaces::shm_pool::{wl_shm_pool, wl_shm_pool_destroy, wl_shm_pool_resize};
use ffi::FFI;

/// A shared memory pool.
///
/// It represents a chunk of memory shared between your program and the
/// wayland client. You can write to it using the means you used to create
/// it (for example by writing to the file if you used a temporary file).
///
/// This pool can the be split into buffers, which are "views" into the pool
/// that the server will use to draw on the surfaces.
pub struct ShmPool {
    _shm: Shm,
    ptr: *mut wl_shm_pool
}

// ShmPool is self owned
unsafe impl Send for ShmPool {}
// The wayland library guaranties this.
unsafe impl Sync for ShmPool {}

impl ShmPool {
    /// Creates a new buffer from this memory pool.
    ///
    /// - `offset` is the number of bytes to skip from the beginning of the pool.
    /// - `width` and `height` are the dimensions of the image the server will read.
    /// - `stride`  is the number of bytes separating the begining of each line (
    ///   for example, on a ARGB888 format, each pixel is 4 bytes long, so on a
    ///   classic data layout we would have `stride = 4*width`).
    /// - `format` is the format of the data contained in the buffer.
    pub fn create_buffer<'b>(&self, offset: i32, width: i32, height: i32, stride: i32, format: ShmFormat)
            -> Option<Buffer> {
        FromOpt::from((self, offset, width, height, stride, format as u32))
    }

    /// Resizes the memory pool to a new size.
    ///
    /// Can only be used to make the pool bigger
    pub fn resize(&self, size: i32) {
        unsafe { wl_shm_pool_resize(self.ptr, size); }
    }
}

impl From<(Shm, i32, i32)> for ShmPool {
    fn from((shm, fd, size): (Shm, i32, i32)) -> ShmPool {
        let ptr = unsafe { wl_shm_create_pool(shm.ptr_mut(), fd, size) };
        ShmPool {
            _shm: shm,
            ptr: ptr
        }
    }
}

impl Drop for ShmPool {
    fn drop(&mut self) {
        unsafe { wl_shm_pool_destroy(self.ptr) };
    }
}

impl FFI for ShmPool {
    type Ptr = wl_shm_pool;

    fn ptr(&self) -> *const wl_shm_pool {
        self.ptr as *const wl_shm_pool
    }

    unsafe fn ptr_mut(&self) -> *mut wl_shm_pool {
        self.ptr
    }
}

