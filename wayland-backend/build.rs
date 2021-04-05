fn main() {
    // build the client shim
    cc::Build::new().file("src/sys/client/log_shim.c").compile("log_shim_client");
    println!("cargo:rerun-if-changed=src/client/log_shim.c");
    // build the server shim
    cc::Build::new().file("src/sys/server/log_shim.c").compile("log_shim_server");
    println!("cargo:rerun-if-changed=src/server/log_shim.c");
}
