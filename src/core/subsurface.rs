use std::ops::Deref;

use super::{From, SubCompositor, Surface, WSurface};

use ffi::interfaces::subcompositor::wl_subcompositor_get_subsurface;
use ffi::interfaces::subsurface::{wl_subsurface, wl_subsurface_destroy,
                                  wl_subsurface_set_position,
                                  wl_subsurface_set_sync,
                                  wl_subsurface_set_desync};
use ffi::FFI;

/// A wayland subsurface.
///
/// It wraps a surface, to be integrated into a parent surface.
pub struct SubSurface<'p, S: Surface> {
    _subcompositor: SubCompositor,
    ptr: *mut wl_subsurface,
    surface: S,
    _parent: &'p WSurface
}

// SubSurface is self owned
unsafe impl<'p, S: Surface + Send> Send for SubSurface<'p, S> {}
// The wayland library guaranties this.
unsafe impl<'p, S: Surface + Sync> Sync for SubSurface<'p, S> {}

impl<'p, S: Surface> SubSurface<'p, S> {
    /// Frees the `Surface` from its role of `subsurface` and returns it.
    pub fn destroy(mut self) -> S {
        use std::mem::{forget, replace, uninitialized};
        unsafe {
            let surface = replace(&mut self.surface, uninitialized());
            wl_subsurface_destroy(self.ptr);
            forget(self);
            surface
        }
    }

    /// Sets the position of the subsurface in the parent surface coordinate
    /// system.
    ///
    /// The change will tale effect at the next `Surface::commit()` call on the parent
    /// surface.
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe { wl_subsurface_set_position(self.ptr, x, y) }
    }

    /// Sets of unsets the subsurface into synchronysed mode.
    ///
    /// When in synchronised mode, the changes of this surface are not applied when
    /// calling `commit()`, but rather cached, and will be applied when the parents
    /// commit is called.
    ///
    /// When a subsurface is set to synchronised mode, all its children subsurfaces
    /// are forcedinto this mode as well.
    pub fn set_sync(&self, b: bool) {
        if b {
            unsafe { wl_subsurface_set_sync(self.ptr) }
        } else {
            unsafe { wl_subsurface_set_desync(self.ptr) }
        }
    }

}

impl<'p, S: Surface> Deref for SubSurface<'p, S> {
    type Target = S;

    fn deref<'d>(&'d self) -> &'d S {
        &self.surface
    }
}

impl<'p, S> From<(SubCompositor, S, &'p WSurface)> for SubSurface<'p, S>
    where S: Surface
{
    fn from((subcompositor, surface, parent): (SubCompositor, S, &'p WSurface))
        -> SubSurface<'p, S>
    {
        let ptr = unsafe {
            wl_subcompositor_get_subsurface(
                subcompositor.ptr_mut(),
                surface.get_wsurface().ptr_mut(),
                parent.ptr_mut()
            )
        };
        SubSurface {
            _subcompositor: subcompositor,
            ptr: ptr,
            surface: surface,
            _parent: parent,
        }
    }
}

impl<'p, S: Surface> Drop for SubSurface<'p, S> {
    fn drop(&mut self) {
        unsafe { wl_subsurface_destroy(self.ptr) };
    }
}

impl<'p, S: Surface> FFI for SubSurface<'p, S> {
    type Ptr = wl_subsurface;

    fn ptr(&self) -> *const wl_subsurface {
        self.ptr as *const wl_subsurface
    }

    unsafe fn ptr_mut(&self) -> *mut wl_subsurface {
        self.ptr
    }
}