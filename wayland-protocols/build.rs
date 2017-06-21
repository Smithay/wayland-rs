extern crate wayland_scanner;

use std::env::var;
use std::path::Path;

use wayland_scanner::{Side, generate_code, generate_interfaces};

static BASE_PROTOCOL_DIR: &'static str = "./protocols/";

static STABLE_PROTOCOLS: &'static [(&'static str, &'static str)] =
    &[
        (
            "presentation-time",
            "presentation-time/presentation-time.xml",
        ),
        ("viewporter", "viewporter/viewporter.xml"),
    ];

static UNSTABLE_PROTOCOLS: &'static [(&'static str, &'static str)] =
    &[
        (
            "fullscreen-shell",
            "fullscreen-shell/fullscreen-shell-unstable-v1.xml",
        ),
        ("idle-inhibit", "idle-inhibit/idle-inhibit-unstable-v1.xml"),
        ("input-method", "input-method/input-method-unstable-v1.xml"),
        ("linux-dmabuf", "linux-dmabuf/linux-dmabuf-unstable-v1.xml"),
        (
            "pointer-constraints",
            "pointer-constraints/pointer-constraints-unstable-v1.xml",
        ),
        (
            "pointer-gestures",
            "pointer-gestures/pointer-gestures-unstable-v1.xml",
        ),
        (
            "relative-pointer",
            "relative-pointer/relative-pointer-unstable-v1.xml",
        ),
        ("tablet", "tablet/tablet-unstable-v2.xml"),
        ("text-input", "text-input/text-input-unstable-v1.xml"),
        ("xdg-foreign", "xdg-foreign/xdg-foreign-unstable-v1.xml"),
        ("xdg-shell", "xdg-shell/xdg-shell-unstable-v6.xml"),
    ];

fn generate_protocol(name: &str, file: &Path, out_dir: &Path, client: bool, server: bool) {

    let protocol_file = Path::new(BASE_PROTOCOL_DIR).join(file);

    generate_interfaces(
        &protocol_file,
        out_dir.join(&format!("{}_interfaces.rs", name)),
    );

    if client {
        generate_code(
            &protocol_file,
            out_dir.join(&format!("{}_client_api.rs", name)),
            Side::Client,
        );
    }

    if server {
        generate_code(
            &protocol_file,
            out_dir.join(&format!("{}_server_api.rs", name)),
            Side::Server,
        );
    }
}

fn main() {
    let out_dir_str = var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    let client = var("CARGO_FEATURE_CLIENT").ok().is_some();
    let server = var("CARGO_FEATURE_SERVER").ok().is_some();

    for &(name, file) in STABLE_PROTOCOLS {
        generate_protocol(
            name,
            &Path::new("stable").join(file),
            out_dir,
            client,
            server,
        );
    }



    if var("CARGO_FEATURE_UNSTABLE_PROTOCOLS").ok().is_some() {
        for &(name, file) in UNSTABLE_PROTOCOLS {
            generate_protocol(
                name,
                &Path::new("unstable").join(file),
                out_dir,
                client,
                server,
            );
        }
    }
}
