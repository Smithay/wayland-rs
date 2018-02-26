use std::io::Result as IOResult;
use std::cmp;
use std::io::Write;

use common_gen::*;
use protocol::*;
use util::*;
use Side;

pub(crate) fn write_protocol_client<O: Write>(protocol: Protocol, out: &mut O) -> IOResult<()> {
    write_prefix(&protocol, out)?;

    for iface in &protocol.interfaces {
        writeln!(out, "pub mod {} {{", iface.name)?;

        if let Some((ref short, ref long)) = iface.description {
            write_doc(Some(short), long, true, out, 1)?;
        }

        writeln!(
            out,
            "    use super::{{Proxy, NewProxy, AnonymousObject, Interface}};\n"
        )?;
        writeln!(
            out,
            "    use super::sys::common::{{wl_argument, wl_interface, wl_array}};"
        )?;
        writeln!(out, "    use super::sys::client::*;");

        let iface_name = snake_to_camel(&iface.name);

        write_enums(&iface.enums, out)?;
        write_messagegroup("Requests", Side::Client, false, &iface.requests, out)?;
        write_messagegroup_impl(
            "Requests",
            &iface_name,
            Side::Client,
            false,
            &iface.requests,
            out,
        )?;
        write_messagegroup("Events", Side::Client, true, &iface.events, out)?;
        write_messagegroup_impl(
            "Events",
            &iface_name,
            Side::Client,
            true,
            &iface.events,
            out,
        )?;
        write_interface(&iface_name, &iface.name, out)?;

        writeln!(out, "}}\n")?;
    }

    Ok(())
}

pub(crate) fn write_protocol_server<O: Write>(protocol: Protocol, out: &mut O) -> IOResult<()> {
    write_prefix(&protocol, out)?;

    for iface in &protocol.interfaces {
        // display and registry are handled specially
        if iface.name == "wl_display" || iface.name == "wl_registry" {
            continue;
        }

        writeln!(out, "pub mod {} {{", iface.name)?;

        if let Some((ref short, ref long)) = iface.description {
            write_doc(Some(short), long, true, out, 1)?;
        }

        writeln!(
            out,
            "    use super::{{Resource, NewResource, AnonymousObject, Interface}};\n"
        )?;
        writeln!(
            out,
            "    use super::sys::common::{{wl_argument, wl_interface, wl_array}};"
        )?;
        writeln!(out, "    use super::sys::server::*;")?;

        let iface_name = snake_to_camel(&iface.name);

        write_enums(&iface.enums, out)?;
        write_messagegroup("Requests", Side::Server, true, &iface.requests, out)?;
        write_messagegroup_impl(
            "Requests",
            &iface_name,
            Side::Server,
            true,
            &iface.requests,
            out,
        )?;
        write_messagegroup("Events", Side::Server, false, &iface.events, out)?;
        write_messagegroup_impl(
            "Events",
            &iface_name,
            Side::Server,
            false,
            &iface.events,
            out,
        )?;
        write_interface(&iface_name, &iface.name, out)?;

        writeln!(out, "}}\n")?;
    }

    Ok(())
}

pub fn write_messagegroup_impl<O: Write>(
    name: &str,
    iface: &str,
    side: Side,
    receiver: bool,
    messages: &[Message],
    out: &mut O,
) -> IOResult<()> {
    writeln!(out, "    impl super::MessageGroup for {} {{", name)?;

    // is_destructor
    writeln!(out, "        fn is_destructor(&self) -> bool {{")?;
    writeln!(out, "            match *self {{")?;
    for msg in messages {
        if msg.typ == Some(Type::Destructor) {
            write!(
                out,
                "                {}::{} ",
                name,
                snake_to_camel(&msg.name)
            )?;
            if msg.args.len() > 0 {
                write!(out, "{{ .. }} ")?;
            }
            writeln!(out, "=> true,")?;
        }
    }
    writeln!(out, "                _ => false")?;
    writeln!(out, "            }}")?;
    writeln!(out, "        }}\n")?;

    // from_raw_c
    writeln!(out, "        unsafe fn from_raw_c(obj: *mut ::std::os::raw::c_void, opcode: u32, args: *const wl_argument) -> Result<{},()> {{", name)?;
    if !receiver {
        writeln!(
            out,
            "            panic!(\"{}::from_raw_c can not be used {:?}-side.\")",
            name, side
        )?;
    } else {
        writeln!(out, "            match opcode {{")?;
        for (i, msg) in messages.iter().enumerate() {
            writeln!(out, "                {} => {{", i)?;
            if msg.args.len() > 0 {
                writeln!(
                    out,
                    "                    let _args = ::std::slice::from_raw_parts(args, {});",
                    msg.args.len()
                )?;
            }
            write!(
                out,
                "                    Ok({}::{}",
                name,
                snake_to_camel(&msg.name)
            )?;
            if msg.args.len() > 0 {
                writeln!(out, " {{")?;
                let mut j = 0;
                for a in &msg.args {
                    write!(out, "                        {}: ", a.name)?;
                    match a.typ {
                        Type::Uint => write!(out, "_args[{}].u", j)?,
                        Type::Int => write!(out, "_args[{}].i", j)?,
                        Type::Fixed => write!(out, "(_args[{}].f as f64)/256.", j)?,
                        Type::String => {
                            if a.allow_null {
                                write!(out, "if _args[{}].s.is_null() {{ None }} else {{ Some(", j)?;
                            }
                            write!(
                                out,
                                "::std::ffi::CStr::from_ptr(_args[{}].s).to_string_lossy().into_owned()",
                                j
                            )?;
                            if a.allow_null {
                                write!(out, ") }}")?;
                            }
                        }
                        Type::Array => {
                            if a.allow_null {
                                write!(out, "if _args[{}].a.is_null() {{ None }} else {{ Some(", j)?;
                            }
                            write!(out, "{{ let array = &*_args[{}].a; ::std::slice::from_raw_parts(array.data as *const u8, array.size).to_owned() }}", j)?;
                            if a.allow_null {
                                write!(out, ") }}")?;
                            }
                        }
                        Type::Fd => write!(out, "_args[{}].h", j)?,
                        Type::Object => {
                            if a.allow_null {
                                write!(out, "if _args[{}].o.is_null() {{ None }} else {{ Some(", j)?;
                            }
                            if let Some(ref iface) = a.interface {
                                write!(
                                    out,
                                    "{}::<super::{}::{}>::from_c_ptr(_args[{}].o as *mut _)",
                                    side.object_name(),
                                    iface,
                                    snake_to_camel(iface),
                                    j
                                )?;
                            } else {
                                write!(
                                    out,
                                    "{}::<AnonymousObject>::from_c_ptr(_args[{}].o as *mut _)",
                                    side.object_name(),
                                    j
                                )?;
                            }
                            if a.allow_null {
                                write!(out, ") }}")?;
                            }
                        }
                        Type::NewId => {
                            if a.allow_null {
                                write!(out, "if _args[{}].o.is_null() {{ None }} else {{ Some(", j)?;
                            }
                            if let Some(ref iface) = a.interface {
                                match side {
                                    Side::Client => write!(
                                        out,
                                        "NewProxy::<super::{}::{}>::from_c_ptr(_args[{}].o as *mut _)",
                                        iface,
                                        snake_to_camel(iface),
                                        j
                                    )?,
                                    Side::Server => {
                                        write!(out, "{{ let client = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_client, obj as *mut _); ")?;
                                        write!(out, "let version = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_get_version, obj as *mut _); ")?;
                                        write!(out, "let new_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_resource_create, client, super::{}::{}::c_interface(), version, _args[{}].n);", iface, snake_to_camel(iface), j)?;
                                        write!(
                                            out,
                                            "NewResource::<super::{}::{}>::from_c_ptr(new_ptr) }}",
                                            iface,
                                            snake_to_camel(iface)
                                        )?;
                                    }
                                }
                            } else {
                                // bind-like function
                                write!(out, "panic!(\"Cannot unserialize anonymous new id.\")")?;
                            }
                            if a.allow_null {
                                write!(out, ") }}")?;
                            }
                        }
                        Type::Destructor => panic!("An argument cannot have type \"destructor\"."),
                    }
                    j += 1;
                    writeln!(out, ",")?;
                }
                write!(out, "                }}")?;
            }
            writeln!(out, ") }},")?;
        }
        writeln!(out, "                _ => return Err(())")?;
        writeln!(out, "            }}")?;
    }
    writeln!(out, "        }}\n")?;

    // as_raw_c_in
    writeln!(
        out,
        "        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &[wl_argument]) -> T {{"
    )?;
    if receiver || side == Side::Client {
        writeln!(
            out,
            "            panic!(\"{}::as_raw_c_in can not be used {:?}-side.\")",
            name, side
        )?;
    } else {
        writeln!(out, "            match self {{")?;
        for (i, msg) in messages.iter().enumerate() {
            write!(
                out,
                "                {}::{} ",
                name,
                snake_to_camel(&msg.name)
            )?;
            if msg.args.len() > 0 {
                write!(out, "{{ ")?;
                for a in &msg.args {
                    write!(out, "{}, ", a.name)?;
                }
                write!(out, "}} ")?;
            }
            writeln!(out, "=> {{")?;
            let mut buffer_len = msg.args.len();
            for a in &msg.args {
                if a.typ == Type::NewId && a.interface.is_none() {
                    buffer_len += 2
                }
            }
            writeln!(out, "                    let mut _args_array: [wl_argument; {}] = unsafe {{ ::std::mem::zeroed() }};", buffer_len)?;
            let mut j = 0;
            for a in &msg.args {
                write!(out, "                    ")?;
                match a.typ {
                    Type::Uint => writeln!(out, "_args_array[{}].u = {};", j, a.name)?,
                    Type::Int => writeln!(out, "_args_array[{}].i = {};", j, a.name)?,
                    Type::Fixed => writeln!(out, "_args_array[{}].f = ({} * 256.) as i32;", j, a.name)?,
                    Type::String => {
                        if a.allow_null {
                            writeln!(
                                out,
                                "let _arg_{} = {}.map(|s| ::std::ffi::CString::new(s).unwrap());",
                                j, a.name
                            )?;
                            write!(out, "                    ")?;
                            writeln!(out, "_args_array[{}].s = _arg_{}.map(|s| s.as_ptr()).unwrap_or(::std::ptr::null());", j, j)?;
                        } else {
                            writeln!(
                                out,
                                "let _arg_{} = ::std::ffi::CString::new({}).unwrap();",
                                j, a.name
                            )?;
                            write!(out, "                    ")?;
                            writeln!(out, "_args_array[{}].s = _arg_{}.as_ptr();", j, j);
                        }
                    }
                    Type::Array => {
                        if a.allow_null {
                            writeln!(out, "let _arg_{} = {}.as_ref().map(|vec| wl_array {{ size: vec.len(), alloc: vec.capacity(), data: vec.as_ptr() as *mut_ }});", j, a.name)?;
                            write!(out, "                    ")?;
                            writeln!(out, "_args_array[{}].a = _arg_{}.as_ref().map(|a| a as *const wl_array).unwrap_or(::std::ptr::null());", j, j)?;
                        } else {
                            writeln!(out, "let _arg_{} = wl_array {{ size: {a}.len(), alloc: {a}.capacity(), data: {a}.as_ptr() as *mut _ }};", j, a=a.name)?;
                            write!(out, "                    ")?;
                            writeln!(out, "_args_array[{}].a = &_arg_{};", j, j)?;
                        }
                    }
                    Type::Fd => writeln!(out, "_args_array[{}].h = {};", j, a.name)?,
                    Type::Object => {
                        if a.allow_null {
                            writeln!(out, "_args_array[{}].o = {}.map(|o| o.c_ptr() as *mut _).unwrap_or(::std::ptr::null_mut());", j, a.name)?;
                        } else {
                            writeln!(out, "_args_array[{}].o = {}.c_ptr() as *mut _;", j, a.name)?;
                        }
                    }
                    Type::NewId => {
                        if let Some(ref iface) = a.interface {
                            // serialize like a regular object
                            if a.allow_null {
                                writeln!(out, "_args_array[{}].o = {}.map(|o| o.c_ptr() as *mut _).unwrap_or(::std::ptr::null_mut());", j, a.name)?;
                            } else {
                                writeln!(out, "_args_array[{}].o = {}.c_ptr() as *mut _;", j, a.name)?;
                            }
                        } else {
                            // we don't serialize this sepcial case, its only the registry (at
                            // least for now)
                            panic!("Outgoing untyped newproxies are not supported in events.");
                        }
                    }
                    Type::Destructor => panic!("An argument cannot have type \"destructor\"."),
                }

                j += 1;
            }
            writeln!(out, "                    f({}, &_args_array)", i)?;
            writeln!(out, "                }},")?;
        }
        writeln!(out, "            }}")?;
    }
    writeln!(out, "        }}")?;

    writeln!(out, "    }}\n")?;
    Ok(())
}

fn write_interface<O: Write>(name: &str, low_name: &str, out: &mut O) -> IOResult<()> {
    writeln!(
        out,
        r#"
    pub struct {name};

    impl Interface for {name} {{
        type Requests = Requests;
        type Events = Events;
        const NAME: &'static str = "{low_name}";
        fn c_interface() -> *const wl_interface {{
            unsafe {{ &super::super::c_interfaces::{low_name}_interface }}
        }}
    }}
"#,
        name = name,
        low_name = low_name
    )?;
    Ok(())
}
