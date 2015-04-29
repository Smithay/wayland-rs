use std::rc::Rc;

use super::{From, Registry, ShellSurface, Surface};

use ffi::interfaces::shell::{wl_shell, wl_shell_destroy};
use ffi::{FFI, Bind, abi};

struct InternalShell {
    _registry: Registry,
    ptr: *mut wl_shell
}

/// A handle to a wayland `wl_shell`.
///
/// This reprensent the desktop window. A surface must be bound to
/// it in order to be drawed on screen.
///
/// Like other global objects, this handle can be cloned.
#[derive(Clone)]
pub struct Shell {
    internal: Rc<InternalShell>
}

impl Shell {
    /// Assigns the `shell_surface` role to given surface.
    ///
    /// The surface will now behave as a generic window, see ShellSurface
    /// documentation for more details.
    pub fn get_shell_surface<S>(&self, surface: S)
        -> ShellSurface<S>
        where S: Surface
    {
        From::from((self.clone(), surface))
    }
}

impl Bind<Registry> for Shell {
    fn interface() -> &'static abi::wl_interface {
        abi::WAYLAND_CLIENT_HANDLE.wl_shell_interface
    }

    unsafe fn wrap(ptr: *mut wl_shell, registry: Registry) -> Shell {
        Shell {
            internal: Rc::new(InternalShell {
                _registry: registry,
                ptr: ptr
            })
        }
    }
}

impl Drop for InternalShell {
    fn drop(&mut self) {
        unsafe { wl_shell_destroy(self.ptr) };
    }
}

impl FFI for Shell {
    type Ptr = wl_shell;

    fn ptr(&self) -> *const wl_shell {
        self.internal.ptr as *const wl_shell
    }

    unsafe fn ptr_mut(&self) -> *mut wl_shell {
        self.internal.ptr
    }
}
