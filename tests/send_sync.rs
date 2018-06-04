extern crate wayland_server as ways;
extern crate wayland_client as wayc;

fn ensure_send<I: Send>() {}
fn ensure_sync<I: Sync>() {}
fn ensure_both<I: Send+Sync>() {}

#[test]
fn send_sync_client() {
    ensure_both::<wayc::Display>();
    ensure_both::<wayc::Proxy<::wayc::protocol::wl_callback::WlCallback>>();
}

#[test]
fn send_sync_server() {
    ensure_both::<ways::LoopSignal>();
    ensure_both::<ways::Resource<::ways::protocol::wl_callback::WlCallback>>();
}
