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
    println!("Partial diffs found:");
    let diffs = flatten_diffs(diffs);
    let mut print_idx = diffs
        .iter()
        .enumerate()
        .flat_map(|(i, d)| {
            if let &Difference::Same(_) = d {
                Vec::new().into_iter()
            } else {
                ((i - 3)..(i + 4)).collect::<Vec<usize>>().into_iter()
            }
        })
        .collect::<Vec<_>>();
    print_idx.sort();
    print_idx.dedup();
    let mut last_idx = 0;
    for idx in print_idx {
        if idx != last_idx + 1 {
            println!("\n=== Partial diff ===");
        }
        last_idx = idx;
        match diffs[idx] {
            Difference::Same(ref l) => println!("   {}", l),
            Difference::Add(ref l) => println!("\x1b[92m + {}\x1b[0m", l),
            Difference::Rem(ref l) => println!("\x1b[91m - {}\x1b[0m", l),
        }
    }
}

fn flatten_diffs(diffs: &[Difference]) -> Vec<Difference> {
    diffs
        .iter()
        .flat_map(|d| match *d {
            Difference::Same(ref x) => x.lines()
                .map(Into::<String>::into)
                .map(Difference::Same)
                .collect::<Vec<_>>()
                .into_iter(),
            Difference::Add(ref x) => x.lines()
                .map(Into::<String>::into)
                .map(Difference::Add)
                .collect::<Vec<_>>()
                .into_iter(),
            Difference::Rem(ref x) => x.lines()
                .map(Into::<String>::into)
                .map(Difference::Rem)
                .collect::<Vec<_>>()
                .into_iter(),
        })
        .collect()
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
