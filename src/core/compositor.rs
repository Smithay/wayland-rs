use std::rc::Rc;

use super::{From, Registry, Region, WSurface};

use ffi::interfaces::compositor::{wl_compositor, wl_compositor_destroy};
use ffi::{FFI, Bind, abi};

struct CompositorInternal {
    _registry: Registry,
    ptr: *mut wl_compositor
}

/// A wayland compositor.
///
/// This is the back-end that will be used for all drawing.
///
/// Like other global objects, this handle can be cloned.
#[derive(Clone)]
pub struct Compositor {
    internal: Rc<CompositorInternal>
}

impl Compositor {
    /// Creates a new surface to draw on.
    pub fn create_surface(&self) -> WSurface {
        From::from(self.clone())
    }

    /// Creates a new region.
    pub fn create_region(&self) -> Region {
        From::from(self.clone())
    }
}

impl Bind<Registry> for Compositor {
    fn interface() -> &'static abi::wl_interface {
        abi::WAYLAND_CLIENT_HANDLE.wl_compositor_interface
    }

    unsafe fn wrap(ptr: *mut wl_compositor, registry: Registry) -> Compositor {
        Compositor {
            internal: Rc::new(CompositorInternal { 
                _registry: registry,
                ptr: ptr
            })
        }
    }
}

impl Drop for CompositorInternal {
    fn drop(&mut self) {
        unsafe { wl_compositor_destroy(self.ptr) };
    }
}

impl FFI for Compositor {
    type Ptr = wl_compositor;

    fn ptr(&self) -> *const wl_compositor {
        self.internal.ptr as *const wl_compositor
    }

    unsafe fn ptr_mut(&self) -> *mut wl_compositor {
        self.internal.ptr
    }
}