extern crate wayland_rust_scanner as scanner;

use std::env;
use std::path::Path;

fn main() {
    let protocols_dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("protocols");
    let out_dir_str = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    // wayland.xml
    scanner::generate(
        scanner::Action::Interfaces,
        protocols_dir.join("wayland.xml"),
        out_dir.join("wayland_interfaces.rs")
    );
    scanner::generate(
        scanner::Action::ClientAPI,
        protocols_dir.join("wayland.xml"),
        out_dir.join("wayland_client_api.rs")
    );
}
