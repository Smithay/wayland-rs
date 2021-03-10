#[cfg(feature = "client")]
pub mod client;

/// Magic static for wayland objects managed by wayland-client or wayland-server
///
/// This static serves no purpose other than existing at a stable address.
pub static RUST_MANAGED: u8 = 42;
