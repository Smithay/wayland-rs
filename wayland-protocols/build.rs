extern crate wayland_scanner;

use std::env::var;
use std::path::Path;
use wayland_scanner::*;

#[rustfmt::skip]
type StableProtocol<'a> =    (&'a str,                &'a [(&'a str, &'a str)]);
type VersionedProtocol<'a> = (&'a str, &'a [(&'a str, &'a [(&'a str, &'a str)])]);
//                            ^        ^         ^        ^     ^        ^
//                            |        |         |        |     |        |
//                            Name     |         |        |     |        Name of event to specify as
//                                     Versions  |        |     |        destructor
//                                               Version  |     |
//                                                        |     Interface the event is belongs to
//                                                        |
//                                                        Events to specify as destructors

static STABLE_PROTOCOLS: &[StableProtocol] =
    &[("presentation-time", &[]), ("viewporter", &[]), ("xdg-shell", &[])];

static STAGING_PROTOCOLS: &[VersionedProtocol] = &[("xdg-activation", &[("v1", &[])])];

static UNSTABLE_PROTOCOLS: &[VersionedProtocol] = &[
    ("fullscreen-shell", &[("v1", &[])]),
    ("idle-inhibit", &[("v1", &[])]),
    ("input-method", &[("v1", &[])]),
    ("input-timestamps", &[("v1", &[])]),
    ("keyboard-shortcuts-inhibit", &[("v1", &[])]),
    ("linux-dmabuf", &[("v1", &[])]),
    (
        "linux-explicit-synchronization",
        &[(
            "v1",
            &[
                ("zwp_linux_buffer_release_v1", "fenced_release"),
                ("zwp_linux_buffer_release_v1", "immediate_release"),
            ],
        )],
    ),
    ("pointer-constraints", &[("v1", &[])]),
    ("pointer-gestures", &[("v1", &[])]),
    ("primary-selection", &[("v1", &[])]),
    ("relative-pointer", &[("v1", &[])]),
    ("tablet", &[("v1", &[]), ("v2", &[])]),
    ("text-input", &[("v1", &[]), ("v3", &[])]),
    ("xdg-decoration", &[("v1", &[])]),
    ("xdg-foreign", &[("v1", &[]), ("v2", &[])]),
    ("xdg-output", &[("v1", &[])]),
    ("xdg-shell", &[("v5", &[]), ("v6", &[])]),
    ("xwayland-keyboard-grab", &[("v1", &[])]),
];

static WLR_UNSTABLE_PROTOCOLS: &[VersionedProtocol] = &[
    ("wlr-data-control", &[("v1", &[])]),
    ("wlr-export-dmabuf", &[("v1", &[])]),
    ("wlr-foreign-toplevel-management", &[("v1", &[])]),
    ("wlr-gamma-control", &[("v1", &[])]),
    ("wlr-input-inhibitor", &[("v1", &[])]),
    ("wlr-layer-shell", &[("v1", &[])]),
    ("wlr-output-management", &[("v1", &[])]),
    ("wlr-output-power-management", &[("v1", &[])]),
    ("wlr-screencopy", &[("v1", &[])]),
    ("wlr-virtual-pointer", &[("v1", &[])]),
];

static MISC_PROTOCOLS: &[StableProtocol] = &[
    ("gtk-primary-selection", &[]),
    ("input-method-unstable-v2", &[]),
    ("server-decoration", &[]),
];

fn generate_protocol(
    name: &str,
    protocol_file: &Path,
    out_dir: &Path,
    client: bool,
    server: bool,
    dest_events: &[(&str, &str)],
) {
    println!("cargo:rerun-if-changed={}", protocol_file.display());

    if client {
        generate_code_with_destructor_events(
            &protocol_file,
            out_dir.join(&format!("{}_client_api.rs", name)),
            Side::Client,
            dest_events,
        );
    }
    if server {
        generate_code_with_destructor_events(
            &protocol_file,
            out_dir.join(&format!("{}_server_api.rs", name)),
            Side::Server,
            dest_events,
        );
    }
}

fn main() {
    println!("cargo:rerun-if-changed-env=CARGO_FEATURE_CLIENT");
    println!("cargo:rerun-if-changed-env=CARGO_FEATURE_SERVER");
    println!("cargo:rerun-if-changed-env=CARGO_FEATURE_UNSTABLE_PROTOCOLS");

    let out_dir_str = var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    let client = var("CARGO_FEATURE_CLIENT").ok().is_some();
    let server = var("CARGO_FEATURE_SERVER").ok().is_some();

    for &(name, dest_events) in STABLE_PROTOCOLS {
        let file = format!("{name}/{name}.xml", name = name);
        generate_protocol(
            name,
            &Path::new("./protocols/stable").join(&file),
            out_dir,
            client,
            server,
            dest_events,
        );
    }

    if var("CARGO_FEATURE_STAGING_PROTOCOLS").ok().is_some() {
        for &(name, versions) in STAGING_PROTOCOLS {
            for &(version, dest_events) in versions {
                let file = format!("{name}/{name}-{version}.xml", name = name, version = version);
                generate_protocol(
                    &format!("{name}-{version}", name = name, version = version),
                    &Path::new("./protocols/staging").join(&file),
                    out_dir,
                    client,
                    server,
                    dest_events,
                );
            }
        }
    }

    for &(name, dest_events) in MISC_PROTOCOLS {
        let file = format!("{name}.xml", name = name);
        generate_protocol(
            name,
            &Path::new("./misc").join(&file),
            out_dir,
            client,
            server,
            dest_events,
        );
    }

    if var("CARGO_FEATURE_UNSTABLE_PROTOCOLS").ok().is_some() {
        for &(name, versions) in UNSTABLE_PROTOCOLS {
            for &(version, dest_events) in versions {
                let file =
                    format!("{name}/{name}-unstable-{version}.xml", name = name, version = version);
                generate_protocol(
                    &format!("{name}-{version}", name = name, version = version),
                    &Path::new("./protocols/unstable").join(file),
                    out_dir,
                    client,
                    server,
                    dest_events,
                );
            }
        }
        for &(name, versions) in WLR_UNSTABLE_PROTOCOLS {
            for &(version, dest_events) in versions {
                let file = format!("{name}-unstable-{version}.xml", name = name, version = version);
                generate_protocol(
                    &format!("{name}-{version}", name = name, version = version),
                    &Path::new("./wlr-protocols/unstable").join(file),
                    out_dir,
                    client,
                    server,
                    dest_events,
                );
            }
        }
    }
}
