extern crate difference;
extern crate tempfile;
extern crate wayland_scanner;

use std::cmp::{max, min};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use std::process::Command;

use difference::{Changeset, Difference};
use wayland_scanner::Side;

const PROTOCOL: &'static str = include_str!("./scanner_assets/protocol.xml");

const CLIENT_CODE_TARGET: &'static str = include_str!("./scanner_assets/client_code.rs");
const SERVER_CODE_TARGET: &'static str = include_str!("./scanner_assets/server_code.rs");

fn print_diff(diffs: &[Difference]) {
    println!("Partial diffs found:");
    let diffs = flatten_diffs(diffs);
    let n = diffs.len();
    let mut print_idx = diffs
        .iter()
        .enumerate()
        .flat_map(|(i, d)| {
            if let &Difference::Same(_) = d {
                Vec::new().into_iter()
            } else {
                ((max(i, 3) - 3)..(min(i + 4, n)))
                    .collect::<Vec<usize>>()
                    .into_iter()
            }
        })
        .collect::<Vec<_>>();
    print_idx.sort();
    print_idx.dedup();
    let mut last_idx = 0;
    for idx in print_idx {
        if idx != last_idx + 1 {
            let location: usize = diffs[0..idx]
                .iter()
                .filter_map(|d| match d {
                    &Difference::Same(_) | &Difference::Rem(_) => Some(1),
                    &Difference::Add(_) => None,
                })
                .sum();
            println!("\n=== Partial diff starting at line {} ===", location + 1);
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
            Difference::Same(ref x) => x
                .split("\n")
                .map(Into::<String>::into)
                .map(Difference::Same)
                .collect::<Vec<_>>()
                .into_iter(),
            Difference::Add(ref x) => x
                .split("\n")
                .map(Into::<String>::into)
                .map(Difference::Add)
                .collect::<Vec<_>>()
                .into_iter(),
            Difference::Rem(ref x) => x
                .split("\n")
                .map(Into::<String>::into)
                .map(Difference::Rem)
                .collect::<Vec<_>>()
                .into_iter(),
        })
        .collect()
}

fn run_codegen_test(generated_file_path: &Path, expected_output: &str) {
    match Command::new("rustfmt")
        .arg("--config-path")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg(generated_file_path)
        .status()
    {
        Ok(status) if status.success() => {
            let mut file = File::open(generated_file_path).unwrap();
            let mut actual_output = String::new();
            file.read_to_string(&mut actual_output).unwrap();

            let changeset = Changeset::new(expected_output, &actual_output, "\n");
            if changeset.distance != 0 {
                print_diff(&changeset.diffs);
                panic!(
                    "Scanner output does not match expected output: d = {}",
                    changeset.distance
                );
            }
        }
        _ => {
            println!("Skipped test because rustfmt is not available!");
        }
    }
}

#[test]
fn client_code_generation() {
    let mut tempfile = tempfile::NamedTempFile::new().unwrap();
    wayland_scanner::generate_code_streams(Cursor::new(PROTOCOL.as_bytes()), &mut tempfile, Side::Client);
    run_codegen_test(tempfile.path(), CLIENT_CODE_TARGET);
}

#[test]
fn server_code_generation() {
    let mut tempfile = tempfile::NamedTempFile::new().unwrap();
    wayland_scanner::generate_code_streams(Cursor::new(PROTOCOL.as_bytes()), &mut tempfile, Side::Server);
    run_codegen_test(tempfile.path(), SERVER_CODE_TARGET);
}
