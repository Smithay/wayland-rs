use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};

use super::{From, Registry, ShmPool};

use ffi::interfaces::shm::{wl_shm, wl_shm_destroy};
use ffi::{FFI, Bind, abi};

pub use ffi::enums::wl_shm_format as ShmFormat;

struct InternalShm {
    _registry: Registry,
    ptr: *mut wl_shm
}

// InternalShm is self-owned
unsafe impl Send for InternalShm {}

/// The shared memory controller.
///
/// This object can be queried for memory_pools used for the buffers.
///
/// Like other global objects, this handle can be cloned.
#[derive(Clone)]
pub struct Shm {
    internal: Arc<Mutex<InternalShm>>
}

impl Shm {
    /// Creates a shared memory pool from given file descriptor.
    ///
    /// The server will internally `mmap` the sepcified `size` number of bytes from
    /// this file descriptor.
    /// The created ShmPool will have access to these bytes exactly.
    pub fn pool_from_fd<F: AsRawFd>(&self, fd: &F, size: i32) -> ShmPool {
        From::from((self.clone(), fd.as_raw_fd(), size))
    }
}

impl Bind<Registry> for Shm {
    fn interface() -> &'static abi::wl_interface {
        abi::WAYLAND_CLIENT_HANDLE.wl_shm_interface
    }

    unsafe fn wrap(ptr: *mut wl_shm, registry: Registry) -> Shm {
        Shm {
            internal: Arc::new(Mutex::new(InternalShm {
                _registry:registry,
                ptr: ptr
            }))
        }
    }
}

impl Drop for InternalShm {
    fn drop(&mut self) {
        unsafe { wl_shm_destroy(self.ptr) };
    }
}

impl FFI for Shm {
    type Ptr = wl_shm;
   fn ptr(&self) -> *const wl_shm {
        self.internal.lock().unwrap().ptr as *const wl_shm
    }

    unsafe fn ptr_mut(&self) -> *mut wl_shm {
        self.internal.lock().unwrap().ptr
    }
}
