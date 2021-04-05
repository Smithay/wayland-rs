use crate::protocol::ArgumentType;
use wayland_sys::common::{wl_argument, wl_array};

#[cfg(any(test, feature = "client_system"))]
pub mod client;
#[cfg(any(test, feature = "server_system"))]
pub mod server;

/// Magic static for wayland objects managed by wayland-client or wayland-server
///
/// This static serves no purpose other than existing at a stable address.
static RUST_MANAGED: u8 = 42;

unsafe fn free_arrays(signature: &[ArgumentType], arglist: &[wl_argument]) {
    for (typ, arg) in signature.iter().zip(arglist.iter()) {
        if let ArgumentType::Array(_) = typ {
            let _ = Box::from_raw(arg.a as *mut wl_array);
        }
    }
}
