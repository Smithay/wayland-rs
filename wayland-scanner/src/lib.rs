//! Wayland protocol code-generation machinnery
//!
//! This crate provides procedural macros for generating the rust code associated with a
//! Wayland XML protocol specification, for use with the `wayland-client`, `wayland-server`
//! and `wayland-backend` crates.
//!
//! Before trying to use this crate, you may check if the protocol extension you want to use
//! is not already exposed in the `wayland-protocols` crate.
//!
//! ## Example usage
//!
//! Below is a template for generating the code for a custom protocol client-side. Server-side
//! is identical, just replacing `client` by `server`. The path to the XML file is relative to the
//! crate root.
//!
//! ```rust,ignore
//! // Generate the bindings in their own module
//! pub mod my_protocol {
//!     use wayland_client;
//!     // import objects from the core protocol if needed
//!     use wayland_client::protocol::*;
//!
//!     // This module hosts a low-level representation of the protocol objects
//!     // you will not need to interact with it yourself, but the code generated
//!     // by the generate_client_code! macro will use it
//!     pub mod __interfaces {
//!         // import the interfaces from the core protocol if needed
//!         use wayland_client::protocol::__interfaces::*;
//!         wayland_scanner::generate_interfaces!("./path/to/the/protocol.xml");
//!     }
//!     use self::__interfaces::*;
//!
//!     // This macro generates the actual types that represent the wayland objects of
//!     // your custom protocol
//!     wayland_scanner::generate_client_code!("./path/to/the/protocol.xml");
//! }
//! ```

use std::{ffi::OsString, path::PathBuf};

mod c_interfaces;
mod client_gen;
mod common;
mod interfaces;
mod parse;
mod protocol;
mod server_gen;
mod util;

// `bytes`, `next_chr`, `parse_lit_str`, `parse_lit_str_cooked` and `parse_lit_str_raw` are adapted
// from syn:
// https://github.com/dtolnay/syn/blob/362ee2d02df3f1b2e74c7b7a4cf2ed3c106404c9/src/lit.rs#L1062-L1167
// and
// https://github.com/dtolnay/syn/blob/362ee2d02df3f1b2e74c7b7a4cf2ed3c106404c9/src/lit.rs#L1327-L1388

/// Get the byte at offset idx, or a default of `b'\0'` if we're looking
/// past the end of the input buffer.
fn byte(s: &str, idx: usize) -> u8 {
    if idx < s.len() {
        s.as_bytes()[idx]
    } else {
        0
    }
}

fn next_chr(s: &str) -> char {
    s.chars().next().unwrap_or('\0')
}

// Returns (content, suffix).
fn parse_lit_str(s: &str) -> String {
    match byte(s, 0) {
        b'"' => parse_lit_str_cooked(s),
        b'r' => parse_lit_str_raw(s),
        _ => unreachable!(),
    }
}

// Clippy false positive
// https://github.com/rust-lang-nursery/rust-clippy/issues/2329
#[allow(clippy::needless_continue)]
fn parse_lit_str_cooked(mut s: &str) -> String {
    assert_eq!(byte(s, 0), b'"');
    s = &s[1..];

    let mut content = String::new();
    'outer: loop {
        let ch = match byte(s, 0) {
            b'"' => break,
            b'\\' => {
                let b = byte(s, 1);
                s = &s[2..];
                match b {
                    b'x' => {
                        let (byte, rest) = backslash_x(s);
                        s = rest;
                        assert!(byte <= 0x80, "Invalid \\x byte in string literal");
                        char::from_u32(u32::from(byte)).unwrap()
                    }
                    b'u' => {
                        let (chr, rest) = backslash_u(s);
                        s = rest;
                        chr
                    }
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'\\' => '\\',
                    b'0' => '\0',
                    b'\'' => '\'',
                    b'"' => '"',
                    b'\r' | b'\n' => loop {
                        let ch = next_chr(s);
                        if ch.is_whitespace() {
                            s = &s[ch.len_utf8()..];
                        } else {
                            continue 'outer;
                        }
                    },
                    b => panic!("unexpected byte {:?} after \\ character in byte literal", b),
                }
            }
            b'\r' => {
                assert_eq!(byte(s, 1), b'\n', "Bare CR not allowed in string");
                s = &s[2..];
                '\n'
            }
            _ => {
                let ch = next_chr(s);
                s = &s[ch.len_utf8()..];
                ch
            }
        };
        content.push(ch);
    }

    assert!(s.starts_with('"'));
    content
}

fn parse_lit_str_raw(mut s: &str) -> String {
    assert_eq!(byte(s, 0), b'r');
    s = &s[1..];

    let mut pounds = 0;
    while byte(s, pounds) == b'#' {
        pounds += 1;
    }
    assert_eq!(byte(s, pounds), b'"');
    let close = s.rfind('"').unwrap();
    for end in s[close + 1..close + 1 + pounds].bytes() {
        assert_eq!(end, b'#');
    }

    s[pounds + 1..close].to_owned()
}

fn backslash_x(s: &str) -> (u8, &str) {
    let mut ch = 0;
    let b0 = byte(s, 0);
    let b1 = byte(s, 1);
    ch += 0x10
        * match b0 {
            b'0'..=b'9' => b0 - b'0',
            b'a'..=b'f' => 10 + (b0 - b'a'),
            b'A'..=b'F' => 10 + (b0 - b'A'),
            _ => panic!("unexpected non-hex character after \\x"),
        };
    ch += match b1 {
        b'0'..=b'9' => b1 - b'0',
        b'a'..=b'f' => 10 + (b1 - b'a'),
        b'A'..=b'F' => 10 + (b1 - b'A'),
        _ => panic!("unexpected non-hex character after \\x"),
    };
    (ch, &s[2..])
}

fn backslash_u(mut s: &str) -> (char, &str) {
    if byte(s, 0) != b'{' {
        panic!("{}", "expected { after \\u");
    }
    s = &s[1..];

    let mut ch = 0;
    let mut digits = 0;
    loop {
        let b = byte(s, 0);
        let digit = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => 10 + b - b'a',
            b'A'..=b'F' => 10 + b - b'A',
            b'_' if digits > 0 => {
                s = &s[1..];
                continue;
            }
            b'}' if digits == 0 => panic!("invalid empty unicode escape"),
            b'}' => break,
            _ => panic!("unexpected non-hex character after \\u"),
        };
        if digits == 6 {
            panic!("overlong unicode escape (must have at most 6 hex digits)");
        }
        ch *= 0x10;
        ch += u32::from(digit);
        digits += 1;
        s = &s[1..];
    }
    assert!(byte(s, 0) == b'}');
    s = &s[1..];

    if let Some(ch) = char::from_u32(ch) {
        (ch, s)
    } else {
        panic!("character code {:x} is not a valid unicode character", ch);
    }
}

// End of code adapted from syn

fn parse_lit_str_token(mut stream: proc_macro::TokenStream) -> String {
    loop {
        let mut iter = stream.into_iter();
        let token = iter.next().expect("expected string argument");
        assert!(iter.next().is_none(), "unexpected trailing token");
        let literal = match token {
            proc_macro::TokenTree::Literal(literal) => literal,
            proc_macro::TokenTree::Group(group) => {
                stream = group.stream();
                continue;
            }
            _ => panic!("expected string argument found `{:?}`", token),
        };
        return parse_lit_str(&literal.to_string());
    }
}

/// Proc-macro for generating low-level interfaces associated with an XML specification
#[proc_macro]
pub fn generate_interfaces(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path: OsString = parse_lit_str_token(stream).into();
    let path = if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let mut buf = PathBuf::from(manifest_dir);
        buf.push(path);
        buf
    } else {
        path.into()
    };
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(e) => panic!("Failed to open protocol file {}: {}", path.display(), e),
    };
    let protocol = parse::parse(file);
    interfaces::generate(&protocol, true).into()
}

/// Proc-macro for generating client-side API associated with an XML specification
#[proc_macro]
pub fn generate_client_code(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path: OsString = parse_lit_str_token(stream).into();
    let path = if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let mut buf = PathBuf::from(manifest_dir);
        buf.push(path);
        buf
    } else {
        path.into()
    };
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(e) => panic!("Failed to open protocol file {}: {}", path.display(), e),
    };
    let protocol = parse::parse(file);
    client_gen::generate_client_objects(&protocol).into()
}

/// Proc-macro for generating server-side API associated with an XML specification
#[proc_macro]
pub fn generate_server_code(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path: OsString = parse_lit_str_token(stream).into();
    let path = if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let mut buf = PathBuf::from(manifest_dir);
        buf.push(path);
        buf
    } else {
        path.into()
    };
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(e) => panic!("Failed to open protocol file {}: {}", path.display(), e),
    };
    let protocol = parse::parse(file);
    server_gen::generate_server_objects(&protocol).into()
}

#[cfg(test)]
fn format_rust_code(code: &str) -> String {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    if let Ok(mut proc) = Command::new("rustfmt")
        .arg("--emit=stdout")
        .arg("--edition=2018")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        //.stderr(Stdio::null())
        .spawn()
    {
        {
            let stdin = proc.stdin.as_mut().unwrap();
            stdin.write_all(code.as_bytes()).unwrap();
        }
        if let Ok(output) = proc.wait_with_output() {
            if output.status.success() {
                return std::str::from_utf8(&output.stdout).unwrap().to_owned();
            }
        }
    }
    panic!("Rustfmt failed!");
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Side {
    /// wayland client applications
    Client,
    /// wayland compositors
    Server,
}
