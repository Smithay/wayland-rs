extern crate wayland_commons as wc;

use std::env;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use wc::smallvec;
use wc::socket::{BufferedSocket, Socket};
use wc::wire::{Argument, ArgumentType, Message, MessageDesc};

fn main() {
    let xdg_dir = env::var_os("XDG_RUNTIME_DIR").unwrap();
    let mut path: PathBuf = xdg_dir.into();
    path.push("wayland-0");

    let socket = UnixStream::connect(path).unwrap();
    let mut socket = BufferedSocket::new(unsafe { Socket::from_raw_fd(socket.into_raw_fd()) });

    socket
        .write_message(&Message {
            sender_id: 1, // wl_display
            opcode: 1,    // get registry
            args: smallvec![
                Argument::NewId(2), // id of the created registry
            ],
        })
        .unwrap();

    socket.flush().unwrap();

    ::std::thread::sleep(::std::time::Duration::from_millis(500)); // sleep 0.5 seconds

    let ret = socket.read_messages(
        |id, opcode| match (id, opcode) {
            (2, 0) => Some(&GLOBAL_EVENT.signature),
            _ => None,
        },
        |msg| {
            println!("{:?}", msg);
            true
        },
    );
    println!("{:?}", ret);
}

/*
 * The registry interface
 */

const GLOBAL_EVENT: MessageDesc = MessageDesc {
    name: "global",
    signature: &[ArgumentType::Uint, ArgumentType::Str, ArgumentType::Uint],
    since: 1,
    destructor: false,
};
