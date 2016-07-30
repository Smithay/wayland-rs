pub mod wayland;

#[cfg(all(feature = "unstable-protocols", feature = "wpu-xdg_shell"))]
pub mod xdg_shell;

#[cfg(feature = "wl-desktop_shell")]
pub mod desktop_shell;
