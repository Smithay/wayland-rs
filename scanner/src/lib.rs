extern crate xml;

use std::fs::{File, OpenOptions};
use std::path::Path;

mod util;
mod parse;
mod protocol;
mod interface_gen;
mod common_gen;
mod client_gen;
mod server_gen;

pub enum Action {
    Interfaces,
    ClientAPI,
    ServerAPI
}

pub fn generate<P1: AsRef<Path>, P2: AsRef<Path>>(action: Action, prot: P1, target: P2) {
    let pfile = File::open(prot).unwrap();
    let protocol = parse::parse_stream(pfile);
    let mut out = OpenOptions::new().write(true).truncate(true).create(true).open(target).unwrap();
    match action {
        Action::Interfaces => interface_gen::generate_interfaces(protocol, &mut out),
        Action::ClientAPI => client_gen::generate_client_api(protocol, &mut out),
        Action::ServerAPI => server_gen::generate_server_api(protocol, &mut out)
    }
}