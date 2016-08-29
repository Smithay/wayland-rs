//! Wayland scanner crate
//!
//!
//! This crate is a rust equivalent of the wayland-scanner tool from the
//! official wayland C library.
//!
//! You can use in your build script to generate the rust code for any
//! wayland protocol file, to use alongside the `wayland_client` and
//! `wayland_server` crate to build your applications.
//!
//! TODO: How to use this crate?
extern crate xml;

use std::fs::{File, OpenOptions};
use std::path::Path;

mod util;
mod parse;
mod protocol;
mod side;
mod interface_gen;
mod code_gen;

pub use side::Side;

fn load_xml<P: AsRef<Path>>(prot: P) -> protocol::Protocol {
    let pfile = File::open(prot).unwrap();
    parse::parse_stream(pfile)
}

/// Generate the interfaces for a protocol
///
/// See this crate toplevel documentation for details.
///
/// Args:
///
/// - `protocol`: a path to the XML file describing the protocol, absolute or relative to
///   the build script using this function.
/// - `target`: the path of the file to store this interfaces in.
pub fn generate_interfaces<P1: AsRef<Path>, P2: AsRef<Path>>(protocol: P1, target: P2) {
    let protocol = load_xml(protocol);
    let mut out = OpenOptions::new().write(true).truncate(true).create(true).open(target).unwrap();
    interface_gen::generate_interfaces(protocol, &mut out);
}

/// Generate the code for a protocol
///
/// See this crate toplevel documentation for details.
///
/// Args:
///
/// - `protocol`: a path to the XML file describing the protocol, absolute or relative to
///   the build script using this function.
/// - `target`: the path of the file to store the code in.
/// - `side`: the side (client or server) to generate code for.
pub fn generate_code<P1: AsRef<Path>, P2: AsRef<Path>>(prot: P1, target: P2, side: Side) {
    let protocol = load_xml(prot);
    let mut out = OpenOptions::new().write(true).truncate(true).create(true).open(target).unwrap();
    code_gen::write_protocol(protocol, &mut out, side).unwrap()
}
