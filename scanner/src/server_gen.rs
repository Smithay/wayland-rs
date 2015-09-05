use std::io::Write;

use protocol::*;

pub fn generate_server_api<O: Write>(protocol: Protocol, out: &mut O) {
    writeln!(out, "//\n// This file was auto-generated, do not edit directly\n//\n").unwrap();

    if let Some(text) = protocol.copyright {
        writeln!(out, "/*\n{}\n*/\n", text).unwrap();
    }
    
}