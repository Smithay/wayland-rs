use super::{From, Buffer, Compositor, OutputTransform, Region};

use ffi::interfaces::compositor::wl_compositor_create_surface;
use ffi::interfaces::surface::{wl_surface, wl_surface_destroy, wl_surface_attach,
                               wl_surface_commit, wl_surface_damage,
                               wl_surface_set_opaque_region,
                               wl_surface_set_buffer_transform,
                               wl_surface_set_buffer_scale};
use ffi::FFI;

/// A wayland Surface.
///
/// This is the basic drawing surface. A surface needs to be assigned
/// a role and a buffer to be properly drawn on screen.
pub struct WSurface<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_surface
}

impl<'a> WSurface<'a> {
    /// Attaches given buffer to be the content of the image.
    ///
    /// The buffer is by the server to display it. If the content of the buffer
    /// change, it should be notified to the server by using the `Surface::damage(..)`
    /// method.
    ///
    /// If the attached buffer is destroyed while still in use, the content of the
    /// window becomes undefined.
    ///
    /// All coordinates are computed relative to the top-left corder of the buffer.
    ///
    /// This state is double-buffered, and require a call to `Surface::commit()` to
    /// be applied.
    pub fn attach(&self, buffer: &Buffer, x: i32, y: i32) {
        unsafe { wl_surface_attach(self.ptr, buffer.ptr_mut(), x, y) }
    }

    /// Commit the changes to the server.
    ///
    /// Atomically apply all the pending changes on this surface, on the order in which
    /// they were requested.
    pub fn commit(&self) {
        unsafe { wl_surface_commit(self.ptr) }
    }

    /// Mark part of this surface as damaged.
    ///
    /// Damaged area will be repainted by the server. This can be used to
    /// notify the server about a change in the buffer contents.
    ///
    /// (x, y) are he coordinate of the top-left corner.
    ///
    /// This state is double-buffered, and require a call to `Surface::commit()` to
    /// be applied.
    pub fn damage(&self, x:i32, y:i32, width: i32, height: i32) {
        unsafe { wl_surface_damage(self.ptr, x, y, width, height) }
    }

    /// Sets the opaque region of this surface.
    ///
    /// Marking part of a region as opaque allow the compositer to make optimisations
    /// on the drawing process (a window behind an opaque region does not need to be
    /// drawn).
    ///
    /// Marking as opaque a region that is actually transparent in the buffer data
    /// can cause drawing artifacts.
    ///
    /// By default the surface is marked as fully transparent.
    ///
    /// This state is double-buffered, and require a call to `Surface::commit()` to
    /// be applied.
    pub fn set_opaque_region(&self, region: &Region) {
        unsafe { wl_surface_set_opaque_region(self.ptr, region.ptr_mut()) }
    }

    /// Sets the transformation the server will apply to the buffer.
    ///
    /// The default value is `OutputTransform::WL_OUTPUT_TRANSFORM_NORMAL`.
    ///
    /// This state is double-buffered, and require a call to `Surface::commit()` to
    /// be applied.
    pub fn set_buffer_transform(&self, transform: OutputTransform) {
        unsafe { wl_surface_set_buffer_transform(self.ptr, transform as i32) }
    }

    /// Sets the scale the server will apply to the buffer.
    ///
    /// The drawed data will be of dimensions `(width/scale, height/scale)`.
    ///
    /// Scale must be positive or will be refused by the server.
    ///
    /// This state is double-buffered, and require a call to `Surface::commit()` to
    /// be applied.
    pub fn set_buffer_scale(&self, scale: i32) {
        unsafe { wl_surface_set_buffer_scale(self.ptr, scale) }
    }
}

impl<'a, 'b> From<&'a Compositor<'b>> for WSurface<'a> {
    fn from(compositor: &'a Compositor<'b>) -> WSurface<'a> {
        let ptr = unsafe { wl_compositor_create_surface(compositor.ptr_mut()) };
        WSurface {
            _t: ::std::marker::PhantomData,
            ptr: ptr
        }
    }
}

impl<'a> Drop for WSurface<'a> {
    fn drop(&mut self) {
        unsafe { wl_surface_destroy(self.ptr) };
    }
}

impl<'a> FFI<wl_surface> for WSurface<'a> {
    fn ptr(&self) -> *const wl_surface {
        self.ptr as *const wl_surface
    }

    unsafe fn ptr_mut(&self) -> *mut wl_surface {
        self.ptr
    }
}

/// A trait representing whatever can be used a a surface. Protocol extentions
/// surch as EGL can define their own kind of surfaces, but they wrap a `WSurface`.
pub trait Surface<'a> {
    fn get_wsurface(&self) -> &WSurface<'a>;
}

impl<'a> Surface<'a> for WSurface<'a> {
    fn get_wsurface(&self) -> &WSurface<'a> {
        self
    }
}