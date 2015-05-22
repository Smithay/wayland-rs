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
    /// Maps `surface` as a subsurface of `parent`.
    ///
    /// If `parent` is destroyed, the subsurface will not be displayed any more.
    pub fn get_subsurface<S>(&self, surface: S, parent: &WSurface)
        -> SubSurface<S>
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
