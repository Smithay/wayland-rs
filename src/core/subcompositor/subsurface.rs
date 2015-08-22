use std::ops::Deref;

use core::{From, Surface};
use core::ids::SurfaceId;
use core::compositor::WSurface;
use core::subcompositor::SubCompositor;

use ffi::interfaces::subcompositor::wl_subcompositor_get_subsurface;
use ffi::interfaces::subsurface::{wl_subsurface, wl_subsurface_destroy,
                                  wl_subsurface_set_position,
                                  wl_subsurface_set_sync,
                                  wl_subsurface_set_desync,
                                  wl_subsurface_place_above,
                                  wl_subsurface_place_below};
use ffi::FFI;

/// A wayland subsurface.
///
/// It wraps a surface, to be integrated into a parent surface.
pub struct SubSurface<S: Surface> {
    _subcompositor: SubCompositor,
    ptr: *mut wl_subsurface,
    surface: S,
    parent: SurfaceId,
}

// SubSurface is self owned
unsafe impl<S: Surface + Send> Send for SubSurface<S> {}
// The wayland library guaranties this.
unsafe impl<S: Surface + Sync> Sync for SubSurface<S> {}

pub enum Stacking {
    Above,
    Below
}

impl<S: Surface> SubSurface<S> {
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

    /// Sets this surface to be displayed above or below `other`, which
    /// must be one of its siblings (but not itself).
    ///
    /// Panics if `other` does not have the same parent as `self` or if
    /// `other == self`.
    pub fn restack_sibling<R: Surface>(&self, other: &SubSurface<R>, stack: Stacking) {
        assert!(self.parent == other.parent,
            "Cannot restack a subsurface against one that isn't a sibling.");
        assert!(self.get_wsurface().get_id() != other.get_wsurface().get_id(),
            "Cannot restack a subsurface against itself.");
        match stack {
            Stacking::Above => unsafe {
                wl_subsurface_place_above(self.ptr, other.get_wsurface().ptr_mut())
            },
            Stacking::Below => unsafe {
                wl_subsurface_place_below(self.ptr, other.get_wsurface().ptr_mut())
            }
        }
    }

    /// Sets this surface to be displayed above or below `other`, which
    /// must be its parent.
    ///
    /// Panics if `other` is not the parent of `self`.
    ///
    /// Note: this method requires parent to be passed to ensure it is still
    /// alive, as a parent can be destroyed without destroying its children.
    pub fn restack_parent<R: Surface>(&self, other: &R, stack: Stacking) {
        assert!(self.parent == other.get_wsurface().get_id(),
            "Expected the parent of this subsurface.");
        match stack {
            Stacking::Above => unsafe {
                wl_subsurface_place_above(self.ptr, other.get_wsurface().ptr_mut())
            },
            Stacking::Below => unsafe {
                wl_subsurface_place_below(self.ptr, other.get_wsurface().ptr_mut())
            }
        }
    }

}

impl<S: Surface> Deref for SubSurface<S> {
    type Target = S;

    fn deref(&self) -> &S {
        &self.surface
    }
}

impl<'p, S> From<(SubCompositor, S, &'p WSurface)> for SubSurface<S>
    where S: Surface
{
    fn from((subcompositor, surface, parent): (SubCompositor, S, &'p WSurface))
        -> SubSurface<S>
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
            parent: parent.get_id()
        }
    }
}

impl<S: Surface> Drop for SubSurface<S> {
    fn drop(&mut self) {
        unsafe { wl_subsurface_destroy(self.ptr) };
    }
}

impl<S: Surface> FFI for SubSurface<S> {
    type Ptr = wl_subsurface;

    fn ptr(&self) -> *const wl_subsurface {
        self.ptr as *const wl_subsurface
    }

    unsafe fn ptr_mut(&self) -> *mut wl_subsurface {
        self.ptr
    }
}
