extern crate wayland_scanner;

use std::env::var;
use std::path::Path;
use wayland_scanner::{generate_code, generate_interfaces, Side};

fn main() {
    let protocol_file = "./wayland.xml";

    let out_dir_str = var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    generate_code(protocol_file, out_dir.join("wayland_api.rs"), Side::Client);

    generate_interfaces(protocol_file, out_dir.join("wayland_interfaces.rs"));
}
