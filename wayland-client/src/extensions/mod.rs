//! Protocol extensions
//!
//! Each submodule represents a protocol extension, that can be enabled or
//! disabled by its associated cargo feature.
//!
//! Currently available stable extensions:
//!
//! - presentation-time (feature `wp-presentation_time`)
//! - viewporter (feature `wp-viewporter`)
//!
//! Currently available unstable extensions (no stability guarantee is made for them !):
//!
//! - xdg-shell (feature `wp-xdg_shell`)

#[cfg(feature = "wp-presentation_time")]
pub mod presentation_time;

#[cfg(feature = "wp-viewporter")]
pub mod viewporter;

#[cfg(all(feature = "unstable-protocols", feature = "wpu-xdg_shell"))]
pub mod xdg_shell;

#[cfg(feature= "wl-desktop_shell")]
pub mod desktop_shell;
