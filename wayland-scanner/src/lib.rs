extern crate xml;

use std::fs::{File, OpenOptions};
use std::path::Path;

mod util;
mod parse;
mod protocol;
mod interface_gen;
mod code_gen;

#[derive(Copy,Clone)]
pub enum Action {
    Interfaces,
    Code(Side)
}

#[derive(Copy,Clone,PartialEq,Eq)]
pub enum Side {
    Client,
    Server
}

pub fn generate<P1: AsRef<Path>, P2: AsRef<Path>>(action: Action, prot: P1, target: P2) {
    let pfile = File::open(prot).unwrap();
    let protocol = parse::parse_stream(pfile);
    let mut out = OpenOptions::new().write(true).truncate(true).create(true).open(target).unwrap();
    match action {
        Action::Interfaces => interface_gen::generate_interfaces(protocol, &mut out),
        Action::Code(side) => code_gen::write_protocol(protocol, &mut out, side).unwrap()
    }
}
