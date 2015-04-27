use super::{From, ShellSurface, Surface};

use ffi::interfaces::shell::{wl_shell, wl_shell_destroy};
use ffi::{FFI, Bind, abi};

/// A handle to a wayland `wl_shell`.
///
/// This reprensent the desktop window. A surface must be bound to
/// it in order to be drawed on screen.
pub struct Shell<'a> {
    _t: ::std::marker::PhantomData<&'a ()>,
    ptr: *mut wl_shell
}

impl<'a> Shell<'a> {
    /// Assigns the `shell_surface` role to given surface.
    ///
    /// The surface will now behave as a generic window, see ShellSurface
    /// documentation for more details.
    pub fn get_shell_surface<'b, 'c, S>(&'b self, surface: S)
        -> ShellSurface<'b, 'c, S>
        where S: Surface<'c>
    {
        From::from((self, surface))
    }
}

impl<'a, R> Bind<'a, R> for Shell<'a> {
    fn interface() -> &'static abi::wl_interface {
        abi::WAYLAND_CLIENT_HANDLE.wl_shell_interface
    }

    unsafe fn wrap(ptr: *mut wl_shell, _parent: &'a R) -> Shell<'a> {
        Shell {
            _t: ::std::marker::PhantomData,
            ptr: ptr
        }
    }
}

impl<'a> Drop for Shell<'a> {
    fn drop(&mut self) {
        unsafe { wl_shell_destroy(self.ptr_mut()) };
    }
}

impl<'a> FFI for Shell<'a> {
    type Ptr = wl_shell;

    fn ptr(&self) -> *const wl_shell {
        self.ptr as *const wl_shell
    }

    unsafe fn ptr_mut(&self) -> *mut wl_shell {
        self.ptr
    }
}
