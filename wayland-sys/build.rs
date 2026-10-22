use pkg_config::Config;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(dlopen)");
    println!("cargo:rerun-if-env-changed=FORCE_NO_DLOPEN");

    if std::env::var_os("CARGO_FEATURE_DLOPEN").is_some()
        && std::env::var("FORCE_NO_DLOPEN").map_or(true, |v| v != "1")
    {
        println!("cargo:rustc-cfg=dlopen");
        // Do not link to anything
        return;
    }

    if std::env::var_os("CARGO_FEATURE_CLIENT").is_some() {
        Config::new().probe("wayland-client").unwrap();
    }
    if std::env::var_os("CARGO_FEATURE_CURSOR").is_some() {
        Config::new().probe("wayland-cursor").unwrap();
    }
    if std::env::var_os("CARGO_FEATURE_EGL").is_some() {
        Config::new().probe("wayland-egl").unwrap();
    }
    if std::env::var_os("CARGO_FEATURE_SERVER").is_some() {
        Config::new().probe("wayland-server").unwrap();
    }
}
