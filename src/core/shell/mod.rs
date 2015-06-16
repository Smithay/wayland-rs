//! Structures related to the shell.
//!
//! The `Shell` represents the window manager. It allows you to create
//! `ShellSurface`s out of `Surfaces`, which is what would be usually considered
//! as a "window" in classic situations.
//!
//! Note that many wayland compositor won't draw the window decorations for you.

pub use self::shell::Shell;
pub use self::shell_surface::{ShellSurface, ShellFullscreenMethod};

pub use self::shell_surface::ShellSurfaceResize;

mod shell;
mod shell_surface;