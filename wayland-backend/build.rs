fn main() {
    if std::env::var("CARGO_FEATURE_LOG").ok().is_some() {
        if std::env::var("CARGO_FEATURE_CLIENT_SYSTEM").ok().is_some() {
            // build the client shim
            cc::Build::new().file("src/sys/client_impl/log_shim.c").compile("log_shim_client");
            println!("cargo:rerun-if-changed=src/sys/client_impl/log_shim.c");
        }
        if std::env::var("CARGO_FEATURE_SERVER_SYSTEM").ok().is_some() {
            // build the server shim
            cc::Build::new().file("src/sys/server_impl/log_shim.c").compile("log_shim_server");
            println!("cargo:rerun-if-changed=src/sys/server_impl/log_shim.c");
        }
    }
}
