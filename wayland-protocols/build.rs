extern crate wayland_scanner;

use std::env::var;
use std::path::Path;
use wayland_scanner::{generate_code, generate_interfaces, Side};

static BASE_PROTOCOL_DIR: &'static str = "./protocols/";

static STABLE_PROTOCOLS: &'static [&'static str] = &["presentation-time", "viewporter"];

static UNSTABLE_PROTOCOLS: &'static [(&'static str, &'static [&'static str])] = &[
    ("fullscreen-shell", &["v1"]),
    ("idle-inhibit", &["v1"]),
    ("input-method", &["v1"]),
    ("keyboard-shortcuts-inhibit", &["v1"]),
    ("linux-dmabuf", &["v1"]),
    ("pointer-constraints", &["v1"]),
    ("pointer-gestures", &["v1"]),
    ("relative-pointer", &["v1"]),
    ("tablet", &["v1", "v2"]),
    ("text-input", &["v1"]),
    ("xdg-foreign", &["v1", "v2"]),
    ("xdg-shell", &["v5", "v6"]),
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

    for name in STABLE_PROTOCOLS {
        let file = format!("{name}/{name}.xml", name = name);
        generate_protocol(
            name,
            &Path::new("stable").join(&file),
            out_dir,
            client,
            server,
        );
    }

    if var("CARGO_FEATURE_UNSTABLE_PROTOCOLS").ok().is_some() {
        for &(name, versions) in UNSTABLE_PROTOCOLS {
            for version in versions {
                let file = format!(
                    "{name}/{name}-unstable-{version}.xml",
                    name = name,
                    version = version
                );
                generate_protocol(
                    &format!("{name}-{version}", name = name, version = version),
                    &Path::new("unstable").join(file),
                    out_dir,
                    client,
                    server,
                );
            }
        }
    }
}
