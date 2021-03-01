//! Debugging helpers to handle `WAYLAND_DEBUG` env variable.

use std::{
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

use wayland_commons::Argument;

/// Print the dispatched message to stderr in a following format:
///
/// [timestamp] <- interface@id.msg_name(args)
pub fn print_dispatched_message<Id: Display>(
    interface: &str,
    id: u32,
    msg_name: &str,
    args: &[Argument<Id>],
) {
    // Add timestamp to output.
    print_timestamp();

    eprint!(" <- {}@{}.{}", interface, id, msg_name);

    print_args(args);

    // Add a new line.
    eprintln!();
}

/// Print the send message to stderr in a following format:
///
/// [timestamp] -> interface@id.msg_name(args)
pub fn print_send_message<Id: Display>(
    interface: &str,
    id: u32,
    msg_name: &str,
    args: &[Argument<Id>],
) {
    // Add timestamp to output.
    print_timestamp();

    eprint!(" -> {}@{}.{}", interface, id, msg_name);

    print_args(args);

    // Add a new line.
    eprintln!();
}

/// Print arguments with opening/closing bracket.
fn print_args<Id: Display>(args: &[Argument<Id>]) {
    let num_args = args.len();

    eprint!("(");

    if num_args > 0 {
        // Explicitly handle first argument to handle one arg functions nicely.
        eprint!("{}", args[0]);

        // Handle the rest.
        for arg in args.iter().take(num_args).skip(1) {
            eprint!(", {}", arg);
        }
    }

    eprint!(")")
}

/// Print timestamp in seconds.microseconds format.
fn print_timestamp() {
    if let Ok(timestamp) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let sc = timestamp.as_secs();
        let ms = timestamp.subsec_micros();
        eprint!("[{}.{:06}]", sc, ms);
    }
}
