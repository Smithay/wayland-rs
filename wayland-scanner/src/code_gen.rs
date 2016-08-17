use std::io::Write;
use std::io::Result as IOResult;

use util::*;
use protocol::*;
use Side;

pub fn write_protocol<O: Write>(protocol: Protocol, out: &mut O, side: Side) -> IOResult<()> {
    for iface in &protocol.interfaces {
        try!(write_interface(iface, out, side));
    }
    Ok(())
}

fn write_interface<O: Write>(interface: &Interface, out: &mut O, side: Side) -> IOResult<()> {
    try!(writeln!(out, "pub mod {} {{", interface.name));

    if let Some((ref short, ref long)) = interface.description {
        try!(write_doc(Some(short), long, true, out));
    }

    try!(writeln!(out, "use super::EventQueueHandle;"));
    match side {
        Side::Client => try!(writeln!(out, "use wayland_sys::client::wl_proxy;")),
        Side::Server => try!(writeln!(out, "use wayland_sys::server::wl_resource;"))
    };
    
    try!(writeln!(out, "pub struct {} {{ ptr: *mut {} }}",
        snake_to_camel(&interface.name),
        match side { Side::Client => "wl_proxy", Side::Server => "wl_resource" }
    ));
    
    let to_handle = match side {
        Side::Client => &interface.events,
        Side::Server => &interface.requests
    };
    try!(write_handler_trait(to_handle, out));
    
    try!(writeln!(out, "}}"));
    Ok(())
}

fn write_handler_trait<O: Write>(messages: &[Message], out: &mut O) -> IOResult<()> {
    try!(writeln!(out, "pub trait Handler {{"));
    for msg in messages {
        if let Some((ref short, ref long)) = msg.description {
            try!(write_doc(Some(short), long, false, out));
        }
        try!(write!(out, "fn {}{}(&mut self, evqh: &mut EventQueueHandle",
            if is_keyword(&msg.name) { "_" } else { "" }, msg.name
        ));
        for arg in &msg.args {
            try!(write!(out, ", {}{}: {}",
                if is_keyword(&arg.name) { "_" } else { "" }, arg.name,
                match arg.typ {
                    Type::Object => arg.interface.as_ref()
                                                 .map(|s| format!("&super::{}::{}", s, snake_to_camel(s)))
                                                 .unwrap_or("*mut wl_proxy".into()),
                    _ => arg.typ.rust_type().into()
                }
            ));
        }
        try!(writeln!(out, ");"));
    }    
    try!(writeln!(out, "}}"));
    Ok(())
}

fn write_doc<O: Write>(short: Option<&str>, long: &str, internal: bool, out: &mut O) -> IOResult<()> {
    let p = if internal { '!' } else { '/' };
    if let Some(txt) = short {
        try!(writeln!(out, "//{} {}", p, txt));
        try!(writeln!(out, "//{}", p));
    }
    for l in long.lines() {
        try!(writeln!(out, "//{} {}", p, l.trim()));
    }
    Ok(())
}
