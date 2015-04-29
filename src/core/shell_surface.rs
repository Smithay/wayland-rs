use std::ops::Deref;

use libc::c_void;

use super::{From, Shell, Surface};

use ffi::interfaces::shell::wl_shell_get_shell_surface;
use ffi::interfaces::shell_surface::{wl_shell_surface, wl_shell_surface_destroy,
                                     wl_shell_surface_set_toplevel,
                                     wl_shell_surface_pong, wl_shell_surface_listener,
                                     wl_shell_surface_add_listener};
use ffi::FFI;

/// A wayland `shell_surface`.
///
/// It represents a window in the most generic sense (it can be a
/// regular window, a popup, a full-screen surface, ...).
///
/// A Surface is wrapped inside this object and accessible through
/// `Deref`, so you can use a `ShellSurface` directly to update the
/// uderlying `Surface`.
pub struct ShellSurface<S: Surface> {
    _shell: Shell,
    ptr: *mut wl_shell_surface,
    surface: S
}

impl<S: Surface> ShellSurface<S> {
    /// Frees the `Surface` from its role of `shell_surface` and returns it.
    pub fn destroy(mut self) -> S {
        use std::mem::{forget, replace, uninitialized};
        unsafe {
            let surface = replace(&mut self.surface, uninitialized());
            wl_shell_surface_destroy(self.ptr);
            forget(self);
            surface
        }
    }

    /// Set this shell surface as being a toplevel window.
    ///
    /// It is the most classic window kind.
    pub fn set_toplevel(&self) {
        unsafe { wl_shell_surface_set_toplevel(self.ptr) }
    }
}

impl<S: Surface> Deref for ShellSurface<S> {
    type Target = S;

    fn deref<'c>(&'c self) -> &'c S {
        &self.surface
    }
}

impl<S: Surface> From<(Shell, S)> for ShellSurface<S> {
    fn from((shell, surface): (Shell, S)) -> ShellSurface<S> {
        let ptr = unsafe { wl_shell_get_shell_surface(shell.ptr_mut(), surface.get_wsurface().ptr_mut()) };
        let s = ShellSurface {
            _shell: shell,
            ptr: ptr,
            surface: surface
        };
        unsafe {
            wl_shell_surface_add_listener(s.ptr, &SHELL_SURFACE_LISTENER, ::std::ptr::null_mut());
        }
        s
    }
}

impl<S: Surface> Drop for ShellSurface<S> {
    fn drop(&mut self) {
        unsafe { wl_shell_surface_destroy(self.ptr) };
    }
}

impl<S: Surface> FFI for ShellSurface<S> {
    type Ptr = wl_shell_surface;

    fn ptr(&self) -> *const wl_shell_surface {
        self.ptr as *const wl_shell_surface
    }

    unsafe fn ptr_mut(&self) -> *mut wl_shell_surface {
        self.ptr
    }
}


//
// C-wrappers for the callback closures, to send to wayland
//
extern "C" fn shell_surface_ping(_data: *mut c_void,
                                 shell_surface: *mut wl_shell_surface,
                                 serial: u32
                                ) {
    unsafe { wl_shell_surface_pong(shell_surface, serial) }
}

extern "C" fn shell_surface_configure(_data: *mut c_void,
                                      _shell_surface: *mut wl_shell_surface,
                                      _edges: u32,
                                      _width: i32,
                                      _height: i32
                                     ) {
}

extern "C" fn shell_surface_popup_done(_data: *mut c_void,
                                       _shell_surface: *mut wl_shell_surface,
                                      ) {
}

static SHELL_SURFACE_LISTENER: wl_shell_surface_listener = wl_shell_surface_listener {
    ping: shell_surface_ping,
    configure: shell_surface_configure,
    popup_done: shell_surface_popup_done
};