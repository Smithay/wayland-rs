use std::env::var;

fn main() {
    if var("CARGO_FEATURE_CLIENT").ok().is_some() {
        // build the client shim
        cc::Build::new().file("src/client/log_shim.c").compile("log_shim_client");
        println!("cargo:rerun-if-changed=src/client/log_shim.c");
    }
    if var("CARGO_FEATURE_SERVER").ok().is_some() {
        // build the server shim
        cc::Build::new().file("src/server/log_shim.c").compile("log_shim_server");
        println!("cargo:rerun-if-changed=src/server/log_shim.c");
    }
}
