#![allow(missing_docs)]

pub mod wayland;
#[cfg(all(feature = "unstable-protocols", feature= "wl-desktop_shell"))]

#[cfg(feature = "wp-presentation_time")]
pub mod presentation_time;

#[cfg(feature = "wp-viewporter")]
pub mod viewporter;

#[cfg(all(feature = "unstable-protocols", feature = "wpu-xdg_shell"))]
pub mod xdg_shell;

#[cfg(feature= "wl-desktop_shell")]
pub mod desktop_shell;
