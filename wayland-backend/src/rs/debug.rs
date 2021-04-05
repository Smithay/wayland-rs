//! Debugging helpers to handle `WAYLAND_DEBUG` env variable.

use std::{
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::protocol::Argument;

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

    eprint!(" <- {}@{}.{}, ({})", interface, id, msg_name, DisplaySlice(args));

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

    eprint!(" -> {}@{}.{} ({})", interface, id, msg_name, DisplaySlice(args));

    // Add a new line.
    eprintln!();
}

pub(crate) struct DisplaySlice<'a, D>(pub &'a [D]);

impl<'a, D: Display> Display for DisplaySlice<'a, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut it = self.0.iter();
        if let Some(val) = it.next() {
            write!(f, "{}", val)?;
        }
        for val in it {
            write!(f, ", {}", val)?;
        }
        Ok(())
    }
}

/// Print timestamp in seconds.microseconds format.
fn print_timestamp() {
    if let Ok(timestamp) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let sc = timestamp.as_secs();
        let ms = timestamp.subsec_micros();
        eprint!("[{}.{:06}]", sc, ms);
    }
}
