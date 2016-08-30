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
    try!(writeln!(out, "use super::interfaces::*;"));
    try!(writeln!(out, "use wayland_sys::common::*;"));
    try!(writeln!(out, "use std::ffi::CString;"));
    try!(writeln!(out, "use std::ptr;"));
    match side {
        Side::Client => try!(writeln!(out, "use wayland_sys::client::*;")),
        Side::Server => try!(writeln!(out, "use wayland_sys::server::*;"))
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
    try!(writeln!(out, "fn interface_ptr() -> *const wl_interface {{ unsafe {{ &{}_interface }} }}",
        interface.name
    ));
    try!(writeln!(out, "fn interface_name() -> &'static str {{ \"{}\"  }}",
        interface.name
    ));
    try!(writeln!(out, "fn supported_version() -> u32 {{ {} }}", interface.version));
    try!(writeln!(out, "}}"));


    // client-side events of wl_display are handled by the lib
    if side != Side::Client || interface.name != "wl_display" {
        try!(write_handler_trait(
            match side {
                Side::Client => &interface.events,
                Side::Server => &interface.requests
            },
            out,
            side
        ));
    }

    try!(write_opcodes(
        match side {
            Side::Client => &interface.requests,
            Side::Server => &interface.events
        },
        out,
        &interface.name
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

fn write_opcodes<O: Write>(messages: &[Message], out: &mut O, iname: &str) -> IOResult<()> {
    for (i, msg) in messages.iter().enumerate() {
        try!(writeln!(out, "const {}_{}: u32 = {};",
            snake_to_screaming(&iname),
			snake_to_screaming(&msg.name),
			i
		));
	}
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
            try!(write!(out, ", {}{}: {}{}{}",
                if is_keyword(&arg.name) { "_" } else { "" },
                arg.name,
                if arg.allow_null { "Option<" } else { "" },
                match arg.typ {
                    Type::Object => arg.interface.as_ref()
                                       .map(|s| format!("&super::{}::{}", s, snake_to_camel(s)))
                                       .unwrap_or(format!("*mut {}", side.object_ptr_type())),
                    Type::NewId => arg.interface.as_ref()
                                      .map(|s| format!("super::{}::{}", s, snake_to_camel(s)))
                                      .unwrap_or("(&str, u32)".into()),
                    _ => arg.typ.rust_type().into()
                },
                if arg.allow_null { ">" }  else { "" }
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

        // detect new_id
        let mut newid = None;
        for arg in &msg.args {
            match arg.typ {
                Type::NewId => if newid.is_some() {
                    panic!("Request {}.{} returns more than one new_id", iname, msg.name);
                } else {
                    newid = Some(arg);
                },
                _ => ()
            }
        }

        // method start
        match newid {
            Some(arg) if arg.interface.is_none() && side == Side::Client => {
                try!(write!(out, "pub fn {}{}<T: {}>(&self, version: u32",
                    if is_keyword(&msg.name) { "_" } else { "" },
                    msg.name,
                    side.object_trait()
                ));
            },
            _ => {
                try!(write!(out, "pub fn {}{}(&self",
                    if is_keyword(&msg.name) { "_" } else { "" },
                    msg.name
                ));
            }
        }

        // print args
        for arg in &msg.args {
            try!(write!(out, ", {}{}: {}{}{}",
                if is_keyword(&arg.name) { "_" } else { "" },
                arg.name,
                if arg.allow_null { "Option<" } else { "" },
                match arg.typ {
                    Type::Object => arg.interface.as_ref()
                                       .map(|s| format!("&super::{}::{}", s, snake_to_camel(s)))
                                       .unwrap_or(format!("*mut {}",side.object_ptr_type())),
                    Type::NewId => if side == Side::Server {
                        arg.interface.as_ref()
                           .map(|s| format!("&super::{}::{}", s, snake_to_camel(s)))
                           .unwrap_or(format!("*mut {}",side.object_ptr_type()))
                    } else {
                        // client-side, the return-type handles that
                        continue;
                    },
                    _ => arg.typ.rust_type().into()
                },
                if arg.allow_null { ">" }  else { "" }
            ));
        }
        try!(write!(out, ")"));

        // return type (if newid)
        if side == Side::Client {
        if let Some(arg) = newid {
            try!(write!(out, "-> {}",
                arg.interface.as_ref()
                    .map(|s| format!("super::{}::{}", s, snake_to_camel(s)))
                    .unwrap_or("T".into()),
            ));
        }}
        try!(writeln!(out, " {{"));

        // arg translation for some types
        for arg in &msg.args {
            match arg.typ {
                Type::Fixed => {
                    try!(writeln!(out, "let {0} = wl_fixed_from_double({0});",
                        arg.name
                    ));
                },
                Type::Array => if arg.allow_null {
                    try!(writeln!(out,
"let {0} = {0}.as_ref().map(|v|
    wl_array {{ size: v.len(), alloc: v.capacity(), data: v.as_ptr() as *mut _ }}
);",
                        arg.name
                    ));
                } else {
                    try!(writeln!(out,
"let {0} = wl_array {{ size: {0}.len(), alloc: {0}.capacity(), data: {0}.as_ptr() as *mut _ }};",
                        arg.name
                    ));
                },
                Type::String => if arg.allow_null {
                    try!(writeln!(out,
"let {0} = {0}.map(|s| CString::new(s).unwrap_or_else(|_| panic!(\"Got a String with interior null in {1}.{2}:{0}\")));",
                        arg.name,
                        iname,
                        msg.name
                    ));
                } else {
                    try!(writeln!(out,
"let {0} = CString::new({0}).unwrap_or_else(|_| panic!(\"Got a String with interior null in {1}.{2}:{0}\"));",
                        arg.name,
                        iname,
                        msg.name
                    ));
                },
                _ => ()
            }
        }
       
        // code generation
        if side == Side::Client {
            if let Some(arg) = newid {
                if let Some(ref iface) = arg.interface {
                    try!(write!(out,
                        "let ptr = unsafe {{ ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal_constructor, self.ptr(), {}_{}, &{}_interface",
                        snake_to_screaming(iname),
                        snake_to_screaming(&msg.name),
                        iface
                    ));
                } else {
                    try!(writeln!(out, "if version > <T as Proxy>::supported_version() {{"));
                    try!(writeln!(out, "    panic!(\"Tried to bind interface {{}} with version {{}} while it is only supported up to {{}}.\", <T as Proxy>::interface_name(), version, <T as Proxy>::supported_version())"));
                    try!(writeln!(out, "}}"));
                    try!(write!(out,
                        "let ptr = unsafe {{ ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal_constructor_versioned, self.ptr(), {}_{}, <T as Proxy>::interface_ptr()",
                        snake_to_screaming(iname),
                        snake_to_screaming(&msg.name),
                    ));
                }
            } else {
                try!(write!(out,
                    "unsafe {{ ffi_dispatch!(WAYLAND_CLIENT_HANDLE, wl_proxy_marshal, self.ptr(), {}_{}",
                    snake_to_screaming(iname),
                    snake_to_screaming(&msg.name)
                ));
            }
        } else {
            try!(write!(out,
                "unsafe {{ ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_post_event, self.ptr(), {}_{}",
                snake_to_screaming(iname),
                snake_to_screaming(&msg.name)
            ));
        }

        // write actual args
        for arg in &msg.args {
            match arg.typ {
                Type::NewId if side == Side::Client => {
                    if newid.map(|a| a.interface.is_none()).unwrap() {
                        try!(write!(out, ", (*<T as Proxy>::interface_ptr()).name, version"));
                    }
                    try!(write!(out, ", ptr::null_mut::<wl_proxy>()"));
                },
                Type::String => if arg.allow_null {
                    try!(write!(out,
                        ", {}.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null())",
                        arg.name
                    ));
                } else {
                    try!(write!(out, ", {}.as_ptr()", arg.name));
                },
                Type::Array => if arg.allow_null {
                    try!(write!(out,
                        ", {}.as_ref().map(|a| a as *const wl_array).unwrap_or(ptr::null())",
                        arg.name
                    ));
                } else {
                    try!(write!(out, ", &{} as *const wl_array", arg.name));
                },
                Type::Object => if arg.allow_null {
                    try!(write!(out,
                        ", {}.map({}::ptr).unwrap_or(ptr::null_mut())",
                        arg.name,
                        side.object_trait()
                    ));
                } else {
                    try!(write!(out, ", {}.ptr()", arg.name));
                },
                _ => if arg.allow_null {
                    try!(write!(out, ", {}.unwrap_or(0)", arg.name));
                } else {
                    try!(write!(out, ", {}", arg.name));
                }
            }
        }

        try!(writeln!(out, ") }};"));

        if newid.is_some() && side == Side::Client {
            try!(writeln!(out, "let proxy = unsafe {{ Proxy::from_ptr(ptr) }};"));
            try!(writeln!(out, "proxy"));
        }

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
