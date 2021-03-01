use wayland_commons::Interface;

pub mod client;
pub mod server;

mod debug;
mod map;
mod socket;
mod wire;

#[inline]
fn same_interface(a: &'static Interface, b: &'static Interface) -> bool {
    a as *const Interface == b as *const Interface || a.name == b.name
}
