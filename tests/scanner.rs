extern crate difference;
extern crate wayland_scanner;


use difference::{Changeset, Difference};
use std::io::Cursor;
use std::str::from_utf8;
use wayland_scanner::Side;

const PROTOCOL: &'static str = include_str!("./scanner_assets/protocol.xml");

const C_INTERFACES_TARGET: &'static str = include_str!("./scanner_assets/c_interfaces.rs");

const CLIENT_C_CODE_TARGET: &'static str = include_str!("./scanner_assets/client_c_code.rs");

const SERVER_C_CODE_TARGET: &'static str = include_str!("./scanner_assets/server_c_code.rs");

fn print_diff(diffs: &[Difference]) {
    println!("");
    for d in diffs {
        match *d {
            Difference::Same(ref x) => for l in x.lines() {
                println!("   {}", l);
            },
            Difference::Add(ref x) => for l in x.lines() {
                println!("\x1b[92m + {}\x1b[0m", l);
            },
            Difference::Rem(ref x) => for l in x.lines() {
                println!("\x1b[91m - {}\x1b[0m", l);
            },
        }
    }
}

fn only_newlines_err(diffs: &[Difference]) -> bool {
    for d in diffs {
        match *d {
            Difference::Add(_) | Difference::Rem(_) => return false,
            _ => {}
        }
    }
    return true;
}

#[test]
fn c_interfaces_generation() {
    let mut out = Vec::new();
    wayland_scanner::generate_c_interfaces_streams(Cursor::new(PROTOCOL.as_bytes()), &mut out);
    let changeset = Changeset::new(
        C_INTERFACES_TARGET,
        from_utf8(&out).expect("Output of scanner was not UTF8."),
        "\n",
    );
    if changeset.distance != 0 && !only_newlines_err(&changeset.diffs) {
        print_diff(&changeset.diffs);
        panic!(
            "Scanner output does not match expected output: d = {}",
            changeset.distance
        );
    }
}

#[test]
fn client_c_code_generation() {
    let mut out = Vec::new();
    wayland_scanner::generate_c_code_streams(Cursor::new(PROTOCOL.as_bytes()), &mut out, Side::Client);
    let changeset = Changeset::new(
        CLIENT_C_CODE_TARGET,
        from_utf8(&out).expect("Output of scanner was not UTF8."),
        "\n",
    );
    if changeset.distance != 0 && !only_newlines_err(&changeset.diffs) {
        print_diff(&changeset.diffs);
        panic!(
            "Scanner output does not match expected output: d = {}",
            changeset.distance
        );
    }
}

#[test]
fn server_c_code_generation() {
    let mut out = Vec::new();
    wayland_scanner::generate_c_code_streams(Cursor::new(PROTOCOL.as_bytes()), &mut out, Side::Server);
    let changeset = Changeset::new(
        SERVER_C_CODE_TARGET,
        from_utf8(&out).expect("Output of scanner was not UTF8."),
        "\n",
    );
    if changeset.distance != 0 && !only_newlines_err(&changeset.diffs) {
        print_diff(&changeset.diffs);
        panic!(
            "Scanner output does not match expected output: d = {}",
            changeset.distance
        );
    }
}
