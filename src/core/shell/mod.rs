//! Structures related to the shell

pub use self::shell::Shell;
pub use self::shell_surface::{ShellSurface, ShellFullscreenMethod};

pub use self::shell_surface::ShellSurfaceResize;

mod shell;
mod shell_surface;