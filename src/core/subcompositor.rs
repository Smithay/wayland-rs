use std::sync::Arc;
use std::sync::Mutex;

use super::{From, Registry, Surface, SubSurface, WSurface};

use ffi::interfaces::subcompositor::{wl_subcompositor, wl_subcompositor_destroy};
use ffi::{FFI, Bind, abi};

struct InternalSubCompositor {
    _registry: Registry,
    ptr: *mut wl_subcompositor
}

// InternalSubCompositor is self-owned
unsafe impl Send for InternalSubCompositor {}

/// A wayland subcompositor.
///
/// This is the back-end used to create subsurfaces.
///
/// Like other global objects, this handle can be cloned.
#[derive(Clone)]
pub struct SubCompositor {
    internal : Arc<Mutex<InternalSubCompositor>>
}

impl SubCompositor {
    pub fn get_subsurface<'d, S>(&self, surface: S, parent: &'d WSurface)
        -> SubSurface<'d, S>
        where S: Surface
    {
        From::from((self.clone(), surface, parent))
    }
}

impl Drop for InternalSubCompositor {
    fn drop(&mut self) {
        unsafe { wl_subcompositor_destroy(self.ptr) };
    }
}

impl Bind<Registry> for SubCompositor {
    fn interface() -> &'static abi::wl_interface {
        abi::WAYLAND_CLIENT_HANDLE.wl_subcompositor_interface
    }

    unsafe fn wrap(ptr: *mut wl_subcompositor, registry: Registry) -> SubCompositor {
        SubCompositor {
            internal: Arc::new(Mutex::new(InternalSubCompositor {
                _registry: registry,
                ptr: ptr
            }))
        }
    }
}

impl FFI for SubCompositor {
    type Ptr = wl_subcompositor;

    fn ptr(&self) -> *const wl_subcompositor {
        let internal = self.internal.lock().unwrap();
        internal.ptr as *const wl_subcompositor
    }

    unsafe fn ptr_mut(&self) -> *mut wl_subcompositor {
        let internal = self.internal.lock().unwrap();
        internal.ptr
    }
}