use std::io::Write;
use std::io::Result as IOResult;

use util::*;
use protocol::*;
use Side;

pub fn write_protocol<O: Write>(protocol: Protocol, out: &mut O, side: Side) -> IOResult<()> {
    for iface in &protocol.interfaces {
        if (iface.name == "wl_display" || iface.name == "wl_registry") && side == Side::Server {
            continue
        }
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
    try!(writeln!(out, "use super::{};", side.object_trait()));
    match side {
        Side::Client => try!(writeln!(out, "use wayland_sys::client::wl_proxy;")),
        Side::Server => try!(writeln!(out, "use wayland_sys::server::wl_resource;"))
    };
    
    try!(writeln!(out, "pub struct {} {{ ptr: *mut {} }}",
        snake_to_camel(&interface.name),
        side.object_ptr_type()
    ));

    // Generate object trait impl
    try!(writeln!(out, "impl {} for {} {{",
        side.object_trait(),
        snake_to_camel(&interface.name)
    ));
    try!(writeln!(out, "fn ptr(&self) -> *mut {} {{ self.ptr }}", side.object_ptr_type()));
    try!(writeln!(out, "unsafe fn from_ptr(ptr: *mut {0}) -> {1} {{ {1} {{ ptr: ptr }} }}",
        side.object_ptr_type(),
        snake_to_camel(&interface.name)
    ));
    try!(writeln!(out, "}}"));

    try!(write_handler_trait(
        match side {
            Side::Client => &interface.events,
            Side::Server => &interface.requests
        },
        out,
        side
    ));

    try!(write_impl(
        match side {
            Side::Client => &interface.requests,
            Side::Server => &interface.events
        },
        out,
        &interface.name,
        side
    ));

    try!(writeln!(out, "}}"));
    Ok(())
}

fn write_handler_trait<O: Write>(messages: &[Message], out: &mut O, side: Side) -> IOResult<()> {
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
                                       .unwrap_or(format!("*mut {}",side.object_ptr_type())),
                    Type::NewId => "u32".into(),
                    _ => arg.typ.rust_type().into()
                }
            ));
        }
        try!(writeln!(out, ");"));
    }    
    try!(writeln!(out, "}}"));
    Ok(())
}

fn write_impl<O: Write>(messages: &[Message], out: &mut O, iname: &str, side: Side) -> IOResult<()> {
    try!(writeln!(out, "impl {} {{", snake_to_camel(iname)));
    for msg in messages {
        if let Some((ref short, ref long)) = msg.description {
            try!(write_doc(Some(short), long, false, out));
        }
        try!(write!(out, "pub fn {}{}(&self",
            if is_keyword(&msg.name) { "_" } else { "" }, msg.name
        ));
        for arg in &msg.args {
            try!(write!(out, ", {}{}: {}",
                if is_keyword(&arg.name) { "_" } else { "" }, arg.name,
                match arg.typ {
                    Type::Object => arg.interface.as_ref()
                                       .map(|s| format!("&super::{}::{}", s, snake_to_camel(s)))
                                       .unwrap_or(format!("*mut {}",side.object_ptr_type())),
                    Type::NewId => continue,
                    _ => arg.typ.rust_type().into()
                }
            ));
        }
        try!(write!(out, ")"));
        let mut has_newid = false;
        for arg in &msg.args {
            match arg.typ {
                Type::NewId => if has_newid {
                    panic!("Request {}.{} returns more than one new_id", iname, msg.name);
                } else {
                    has_newid = true;
                    try!(write!(out, "-> {}",
                        arg.interface.as_ref()
                            .map(|s| format!("super::{}::{}", s, snake_to_camel(s)))
                            .unwrap_or(format!("*mut {}", side.object_ptr_type())),
                    ));
                },
                _ => ()
            }
        }
        try!(writeln!(out, " {{"));
        try!(writeln!(out, "unimplemented!();"));
        try!(writeln!(out, "}}"));
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
