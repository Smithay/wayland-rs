extern crate wayland_scanner;

use std::env::var;
use std::path::Path;
use wayland_scanner::{generate_code, generate_interfaces, Side};

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
    ("xdg-output", &["v1"]),
    ("xdg-shell", &["v5", "v6"]),
    ("xwayland-keyboard-grab", &["v1"]),
];

static WALL_STABLE_PROTOCOLS: &'static [&'static str] = &[];

static WALL_UNSTABLE_PROTOCOLS: &'static [(&'static str, &'static [&'static str])] = &[
    ("background", &["v1", "v2"]),
    ("dock-manager", &["v1", "v2"]),
    ("launcher-menu", &["v1"]),
    ("notification-area", &["v1"]),
    ("window-switcher", &["v1"]),
];

fn generate_protocol(name: &str, protocol_file: &Path, out_dir: &Path, client: bool, server: bool) {
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
            &Path::new("./protocols/stable").join(&file),
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
                    &Path::new("./protocols/unstable").join(file),
                    out_dir,
                    client,
                    server,
                );
            }
        }
    }

    if var("CARGO_FEATURE_WALL_PROTOCOLS").ok().is_some() {
        for name in WALL_STABLE_PROTOCOLS {
            let file = format!("{name}/{name}.xml", name = name);
            generate_protocol(
                name,
                &Path::new("./wall/stable").join(&file),
                out_dir,
                client,
                server,
            );
        }

        if var("CARGO_FEATURE_UNSTABLE_PROTOCOLS").ok().is_some() {
            for &(name, versions) in WALL_UNSTABLE_PROTOCOLS {
                for version in versions {
                    let file = format!(
                        "{name}/{name}-unstable-{version}.xml",
                        name = name,
                        version = version
                    );
                    generate_protocol(
                        &format!("{name}-{version}", name = name, version = version),
                        &Path::new("./wall/unstable").join(file),
                        out_dir,
                        client,
                        server,
                    );
                }
            }
        }
    }
}
