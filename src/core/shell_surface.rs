use std::ffi::CStr;
use std::ops::Deref;
use std::ptr;

use libc::c_void;

use super::{From, Output, Shell, Surface};

use ffi::enums::wl_shell_surface_fullscreen_method;
pub use ffi::enums::wl_shell_surface_resize as ShellSurfaceResize;
use ffi::interfaces::shell::wl_shell_get_shell_surface;
use ffi::interfaces::shell_surface::{wl_shell_surface, wl_shell_surface_destroy,
                                     wl_shell_surface_set_toplevel,
                                     wl_shell_surface_pong, wl_shell_surface_listener,
                                     wl_shell_surface_add_listener, wl_shell_surface_set_fullscreen,
                                     wl_shell_surface_set_maximized, wl_shell_surface_set_title,
                                     wl_shell_surface_set_class};
use ffi::FFI;

/// Different methods of fullscreen for a shell surface.
pub enum ShellFullscreenMethod {
    /// Default method: let the compositor decide.
    Default,
    /// Match the sizes by scaling the content of the window to fit
    /// the output dimensions.
    Scale,
    /// Match the sizes by changing the video mode of the graphic driver.
    /// An optionnal framerate can be provided, if not the compositor will it.
    /// The framerate is provided in mHz.
    Driver(Option<u32>),
    /// Buffer is not scaled (but its intrisic scaling is still applied), unless
    /// it is bigger than the output: the compositor is then allowed to scale it down.
    Fill
}

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
    surface: S,
    listener: Box<ShellSurfaceListener>
}

// ShellSurface is self owned
unsafe impl<S: Surface + Send> Send for ShellSurface<S> {}
// The wayland library guaranties this.
unsafe impl<S: Surface + Sync> Sync for ShellSurface<S> {}


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

    /// Set this shell surface as being fullscreen.
    ///
    /// If no output is provided, the compositor will choose the output itself.
    pub fn set_fullscreen(&self, method: ShellFullscreenMethod, output: Option<&Output>) {
        let (wl_method, framerate) = match method {
            ShellFullscreenMethod::Default => (
                wl_shell_surface_fullscreen_method::WL_SHELL_SURFACE_FULLSCREEN_METHOD_DEFAULT,
                0
            ),
            ShellFullscreenMethod::Scale => (
                wl_shell_surface_fullscreen_method::WL_SHELL_SURFACE_FULLSCREEN_METHOD_SCALE,
                0
            ),
            ShellFullscreenMethod::Driver(f) => (
                wl_shell_surface_fullscreen_method::WL_SHELL_SURFACE_FULLSCREEN_METHOD_DRIVER,
                f.unwrap_or(0)
            ),
            ShellFullscreenMethod::Fill => (
                wl_shell_surface_fullscreen_method::WL_SHELL_SURFACE_FULLSCREEN_METHOD_FILL,
                0
            ),
        };
        unsafe { wl_shell_surface_set_fullscreen(
            self.ptr,
            wl_method,
            framerate,
            output.map(|o| o.ptr_mut()).unwrap_or(ptr::null_mut())
        )};
    }

    /// Set this shell surface as being maximised.
    ///
    /// If no output is provided, the compositor will choose the output itself.
    pub fn set_maximised(&self, output: Option<&Output>) {
        unsafe {
            wl_shell_surface_set_maximized(
                self.ptr,
                output.map(|o| o.ptr_mut()).unwrap_or(ptr::null_mut())
            );
        }
    }

    /// Sets the shell surface title.
    ///
    /// This string may be used to identify the surface in a task bar, window list,
    /// or other user interface elements provided by the compositor.
    pub fn set_title(&self, title: &CStr) {
        unsafe {
            wl_shell_surface_set_title(
                self.ptr,
                title.as_ptr()
            );
        }
    }

    /// Sets the shell surface class.
    ///
    /// The surface class identifies the general class of applications to which the
    /// surface belongs. A common convention is to use the file name of the application's
    /// `.desktop` file as the class.
    pub fn set_class(&self, title: &CStr) {
        unsafe {
            wl_shell_surface_set_class(
                self.ptr,
                title.as_ptr()
            );
        }
    }

    /// Sets the callback to be invoked when a `configure` event is received for this shell surface.
    ///
    /// These events are generated then the window is resized, and provide a hint of the new
    /// expected size. It is not a mandatory size, the client can still do has it pleases.
    ///
    /// The arguments of the callback are:
    ///
    ///  - an enum `ShellSurfaceResize`, which is an hint about which border of the surface
    ///    was resized
    ///  - the new `width`
    ///  - the new `height`
    pub fn set_configure_callback<F>(&mut self, f: F)
        where F: Fn(ShellSurfaceResize, i32, i32) + 'static
    {
        self.listener.configure_handler = Box::new(f);
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
        let ptr = unsafe { wl_shell_get_shell_surface(
            shell.ptr_mut(),
            surface.get_wsurface().ptr_mut())
        };
        let listener = ShellSurfaceListener::default_handlers();
        let s = ShellSurface {
            _shell: shell,
            ptr: ptr,
            surface: surface,
            listener: Box::new(listener)
        };
        unsafe {
            wl_shell_surface_add_listener(
                s.ptr,
                &SHELL_SURFACE_LISTENER,
                &*s.listener as *const _ as *mut _
            );
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

/// The data used by the listener callbacks.
struct ShellSurfaceListener {
    /// Handler of the "new global object" event
    configure_handler: Box<Fn(ShellSurfaceResize, i32, i32)>
}

impl ShellSurfaceListener {
    fn default_handlers() -> ShellSurfaceListener {
        ShellSurfaceListener {
            configure_handler: Box::new(move |_, _, _| {})
        }
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

extern "C" fn shell_surface_configure(data: *mut c_void,
                                      _shell_surface: *mut wl_shell_surface,
                                      edges: ShellSurfaceResize,
                                      width: i32,
                                      height: i32
                                     ) {
    let listener = unsafe { &*(data as *const ShellSurfaceListener) };
    (listener.configure_handler)(edges, width,height);
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