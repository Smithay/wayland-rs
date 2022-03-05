//! Backend API for wayland crates

#![warn(missing_docs, missing_debug_implementations)]
#![forbid(improper_ctypes, unsafe_op_in_unsafe_fn)]

pub extern crate smallvec;

/// Helper macro for quickly making a [`Message`](crate::protocol::Message)
#[macro_export]
macro_rules! message {
    ($sender_id: expr, $opcode: expr, [$($args: expr),* $(,)?] $(,)?) => {
        $crate::protocol::Message {
            sender_id: $sender_id,
            opcode: $opcode,
            args: $crate::smallvec::smallvec![$($args),*],
        }
    }
}

#[cfg(any(test, feature = "client_system", feature = "server_system"))]
pub mod sys;

pub mod rs;

#[cfg(not(feature = "client_system"))]
pub use rs::client;
#[cfg(feature = "client_system")]
pub use sys::client;

#[cfg(not(feature = "server_system"))]
pub use rs::server;
#[cfg(feature = "server_system")]
pub use sys::server;

#[cfg(test)]
mod test;

mod core_interfaces;
pub mod protocol;
mod types;

/*
 * These trampoline functions need to always be here because the build script cannot
 * conditionally build their C counterparts on whether the crate is tested or not...
 * They'll be optimized out when unused.
 */

#[no_mangle]
extern "C" fn wl_log_rust_logger_client(msg: *const std::os::raw::c_char) {
    let cstr = unsafe { std::ffi::CStr::from_ptr(msg) };
    let text = cstr.to_string_lossy();
    log::error!("{}", text);
}

#[no_mangle]
extern "C" fn wl_log_rust_logger_server(msg: *const std::os::raw::c_char) {
    let cstr = unsafe { std::ffi::CStr::from_ptr(msg) };
    let text = cstr.to_string_lossy();
    log::error!("{}", text);
}
