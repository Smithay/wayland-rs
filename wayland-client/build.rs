extern crate wayland_scanner;

use std::env::var;
use std::path::Path;

use wayland_scanner::{Action, Side, generate};

fn main() {
    let protocol_file = "./wayland.xml";
    
    let out_dir_str = var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);
    
    println!("Generating files to: {}", out_dir_str);
    
    generate(
        Action::Code(Side::Client),
        protocol_file,
        out_dir.join("wayland_api.rs")
    );
    
    generate(
        Action::Interfaces,
        protocol_file,
        out_dir.join("wayland_interfaces.rs")
    );
    
        
}
