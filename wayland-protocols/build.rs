extern crate wayland_scanner;

use std::env::var;
use std::path::Path;

use wayland_scanner::{Side, generate_code, generate_interfaces};

static BASE_PROTOCOL_DIR: &'static str = "./protocols/";

static STABLE_PROTOCOLS: &'static [(&'static str, &'static str)] = &[
];

static UNSTABLE_PROTOCOLS: &'static [(&'static str, &'static str)] = &[
];

fn generate_protocol(name: &str, file: &str, out_dir: &Path, client: bool, server, bool) {
    
    let protocol_file = Path::new(BASE_PROTOCOL_DIR).join(file);
    
    generate_interfaces(&protocol_file, out_dir.join(&format!("{}_interfaces.rs", name)));
    
    if client {
        generate_code(&protocol_file, out_dir.join(&format!("{}_client_api.rs", name)), Side::Client);
    }
    
    if server {
        generate_code(&protocol_file, out_dir.join(&format!("{}_server_api.rs", name)), Side::Server);
    }
}

fn main() {
    let out_dir_str = var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);
    
    let client = var("CARGO_FEATURE_CLIENT").is_some();
    let server = var("CARGO_FEATURE_SERVER").is_some();
    
    for &(name, file) in STABLE_PROTOCOLS {
        generate_protocol(name, file, out_dir, client, server);
    }
    
    
    
    if var("CARGO_FEATURE_UNSTABLE_PROTOCOLS").is_some() {
        for &(name, file) in UNSTABLE_PROTOCOLS {
            generate_protocol(name, file, out_dir, client, server);
        }
    }
}
