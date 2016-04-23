extern crate xml;

mod util;
mod parse;
mod protocol;
mod interface_gen;
mod common_gen;
mod client_gen;
mod server_gen;

fn main() {

    let mut args = std::env::args();
    args.next();
    let action = match args.next() {
        Some(ref txt) if txt == "interfaces" => 0,
        Some(ref txt) if txt == "client-api" => 1,
        Some(ref txt) if txt == "server-api" => 2,
        _ => {
            println!("Usage:\n wayland-rust-scanner [client-api|server-api|interfaces] < protocol.xml > out_file.rs");
            return;
        }
    };
    let protocol = parse::parse_stream(std::io::stdin());

    if action == 1 {
        // client-api generation
        client_gen::generate_client_api(protocol, &mut std::io::stdout());
    } else if action == 2 {
        // server api-generation
        server_gen::generate_server_api(protocol, &mut std::io::stdout());
    } else {
        // interfaces generation
        interface_gen::generate_interfaces(protocol, &mut std::io::stdout());
    }
}
