//! Backend API for wayland crates
//!
//! This crate provide low-level APIs for interacting with the Wayland protocol,
//! both client-side and server-side.
//!
//! Two possible backends are provided by this crate: the system backend ([`sys`] module)
//! which relies on the system-provided wayland libraries, and the rust backend ([`rs`] module)
//! which is an alternative rust implementation of the protocol. The rust backend is always
//! available, and the system backend is controlled by the `client_system` and `server_system`
//! cargo features. The `dlopen` cargo feature ensures that the system wayland
//! libraries are loaded dynamically at runtime, so that your executable does not link them and
//! can gracefully handle their absence (for example by falling back to X11).
//!
//! Additionnaly the default backends are reexported as toplevel `client` and `server` modules
//! in this crate. For both client and server, the default backend is the system one if the
//! associated cargo feature is enabled, and the rust one otherwise. Using these reexports is the
//! recommended way to use the crate.
//!
//! Both backends have the exact same API, except that the system backend additionnaly provides
//! functions related to FFI.

#![warn(missing_docs, missing_debug_implementations)]
#![forbid(improper_ctypes, unsafe_op_in_unsafe_fn)]
#![cfg_attr(coverage, feature(no_coverage))]

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
