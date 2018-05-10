extern crate wayland_scanner;

use std::env::var;
use std::path::Path;
use wayland_scanner::*;

fn main() {
    let protocol_file = "./wayland.xml";

    let out_dir_str = var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    if var("CARGO_FEATURE_NATIVE_LIB").ok().is_some() {
        // generate the C code
        generate_c_code(protocol_file, out_dir.join("wayland_c_api.rs"), Side::Client);
        generate_c_interfaces(protocol_file, out_dir.join("wayland_c_interfaces.rs"));
    }
}
