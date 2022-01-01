extern crate wayland_client as wayc;
extern crate wayland_server as ways;

fn ensure_both<I: Send + Sync>() {}

#[test]
fn send_sync_client() {
    ensure_both::<wayc::Connection>();
    ensure_both::<wayc::EventQueue<()>>();
    ensure_both::<wayc::protocol::wl_callback::WlCallback>();
}

#[test]
fn send_sync_server() {
    ensure_both::<ways::Display<()>>();
    ensure_both::<ways::protocol::wl_callback::WlCallback>();
    ensure_both::<ways::Client>();
}
