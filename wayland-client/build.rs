extern crate wayland_scanner;

use std::env::var;
use std::path::Path;
use wayland_scanner::*;

fn main() {
    let protocol_file = "./wayland.xml";

    let out_dir_str = var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    println!("cargo:rerun-if-changed={}", protocol_file);
    generate_code_with_destructor_events(
        protocol_file,
        out_dir.join("wayland_api.rs"),
        Side::Client,
        &[("wl_callback", "done")],
    );
}
