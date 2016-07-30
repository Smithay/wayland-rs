extern crate wayland_scanner as scanner;

use std::env::var;
use std::path::Path;

pub fn generate(pdir: &Path, odir: &Path, xml: &str, ifacef: &str, apif: &str) {
    scanner::generate(
        scanner::Action::Interfaces,
        pdir.join(xml),
        odir.join(ifacef)
    );
    scanner::generate(
        scanner::Action::ClientAPI,
        pdir.join(xml),
        odir.join(apif)
    );
}

fn main() {
    let protocols_dir = Path::new(&var("CARGO_MANIFEST_DIR").unwrap()).join("protocols");
    let out_dir_str = var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    // wayland.xml
    generate(&protocols_dir, out_dir,
        "wayland.xml", "wayland_interfaces.rs", "wayland_client_api.rs");

    if var("CARGO_FEATURE_WL_DESKTOP_SHELL").is_ok() {
        generate(&protocols_dir, out_dir,
                 "desktop-shell.xml", "desktop_shell_interfaces.rs", "desktop_shell_api.rs");
    }

    if var("CARGO_FEATURE_WP_PRESENTATION_TIME").is_ok() {
        generate(&protocols_dir, out_dir,
            "presentation-time.xml", "presentation_time_interfaces.rs", "presentation_time_client_api.rs");
    }
    if var("CARGO_FEATURE_WP_VIEWPORTER").is_ok() {
        generate(&protocols_dir, out_dir,
            "viewporter.xml", "viewporter_interfaces.rs", "viewporter_client_api.rs");
    }

    if var("CARGO_FEATURE_UNSTABLE_PROTOCOLS").is_ok() {
        let protocols_dir = protocols_dir.join("unstable");
        // unstable protocols
        if var("CARGO_FEATURE_WPU_XDG_SHELL").is_ok() {
            generate(&protocols_dir, out_dir,
                "xdg-shell-unstable-v5.xml", "xdg_shell_interfaces.rs", "xdg_shell_client_api.rs");
        }
    }
}
