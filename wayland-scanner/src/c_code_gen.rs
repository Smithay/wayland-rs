use std::io::Result as IOResult;
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
            "    use super::{{Proxy, NewProxy, AnonymousObject, Interface, MessageGroup}};\n"
        )?;
        writeln!(
            out,
            "    use super::sys::common::{{wl_argument, wl_interface, wl_array}};"
        )?;
        writeln!(out, "    use super::sys::client::*;")?;

        let iface_name = snake_to_camel(&iface.name);

        write_enums(&iface.enums, out)?;
        write_messagegroup("Request", Side::Client, false, &iface.requests, out)?;
        write_messagegroup_impl("Request", Side::Client, false, &iface.requests, out)?;
        write_messagegroup("Event", Side::Client, true, &iface.events, out)?;
        write_messagegroup_impl("Event", Side::Client, true, &iface.events, out)?;
        write_interface(&iface_name, &iface.name, out)?;
        write_client_methods(&iface_name, &iface.requests, out)?;

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
            "    use super::{{Resource, NewResource, AnonymousObject, Interface, MessageGroup}};\n"
        )?;
        writeln!(
            out,
            "    use super::sys::common::{{wl_argument, wl_interface, wl_array}};"
        )?;
        writeln!(out, "    use super::sys::server::*;")?;

        let iface_name = snake_to_camel(&iface.name);

        write_enums(&iface.enums, out)?;
        write_messagegroup("Request", Side::Server, true, &iface.requests, out)?;
        write_messagegroup_impl("Request", Side::Server, true, &iface.requests, out)?;
        write_messagegroup("Event", Side::Server, false, &iface.events, out)?;
        write_messagegroup_impl("Event", Side::Server, false, &iface.events, out)?;
        write_interface(&iface_name, &iface.name, out)?;

        writeln!(out, "}}\n")?;
    }

    Ok(())
}

pub fn write_messagegroup_impl<O: Write>(
    name: &str,
    side: Side,
    receiver: bool,
    messages: &[Message],
    out: &mut O,
) -> IOResult<()> {
    writeln!(out, "    impl super::MessageGroup for {} {{", name)?;

    // is_destructor
    writeln!(out, "        fn is_destructor(&self) -> bool {{")?;
    writeln!(out, "            match *self {{")?;
    let mut n = messages.len();
    for msg in messages {
        if msg.typ == Some(Type::Destructor) {
            write!(out, "                {}::{} ", name, snake_to_camel(&msg.name))?;
            if msg.args.len() > 0 {
                write!(out, "{{ .. }} ")?;
            }
            writeln!(out, "=> true,")?;
            n -= 1;
        }
    }
    if n > 0 {
        // avoir "unreachable pattern" warnings =)
        writeln!(out, "                _ => false")?;
    }
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
                        Type::Uint => if let Some(ref enu) = a.enum_ {
                            write!(
                                out,
                                "{}::from_raw(_args[{}].u).ok_or(())?",
                                dotted_to_relname(enu),
                                j
                            )?;
                        } else {
                            write!(out, "_args[{}].u", j)?;
                        },
                        Type::Int => if let Some(ref enu) = a.enum_ {
                            write!(
                                out,
                                "{}::from_raw(_args[{}].i as u32).ok_or(())?",
                                dotted_to_relname(enu),
                                j
                            )?;
                        } else {
                            write!(out, "_args[{}].i", j)?;
                        },
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
        "        fn as_raw_c_in<F, T>(self, f: F) -> T where F: FnOnce(u32, &mut [wl_argument]) -> T {{"
    )?;
    if receiver {
        writeln!(
            out,
            "            panic!(\"{}::as_raw_c_in can not be used {:?}-side.\")",
            name, side
        )?;
    } else {
        writeln!(out, "            match self {{")?;
        for (i, msg) in messages.iter().enumerate() {
            write!(out, "                {}::{} ", name, snake_to_camel(&msg.name))?;
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
                    Type::Uint => if a.enum_.is_some() {
                        writeln!(out, "_args_array[{}].u = {}.to_raw();", j, a.name)?;
                    } else {
                        writeln!(out, "_args_array[{}].u = {};", j, a.name)?;
                    },
                    Type::Int => if a.enum_.is_some() {
                        writeln!(out, "_args_array[{}].i = {}.to_raw() as i32;", j, a.name)?;
                    } else {
                        writeln!(out, "_args_array[{}].i = {};", j, a.name)?;
                    },
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
                            writeln!(out, "_args_array[{}].s = _arg_{}.as_ptr();", j, j)?;
                        }
                    }
                    Type::Array => {
                        if a.allow_null {
                            writeln!(out, "let _arg_{} = {}.as_ref().map(|vec| wl_array {{ size: vec.len(), alloc: vec.capacity(), data: vec.as_ptr() as *mut _ }});", j, a.name)?;
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
                        if a.interface.is_some() {
                            writeln!(out, "_args_array[{}].o = {}.c_ptr() as *mut _;", j, a.name)?;
                        } else {
                            if side == Side::Server {
                                panic!("Cannot serialize anonymous NewID from server.");
                            }
                            // The arg is actually (string, uint, NULL)
                            writeln!(
                                out,
                                "let _arg_{}_s = ::std::ffi::CString::new({}.0).unwrap();",
                                j, a.name
                            )?;
                            write!(out, "                    ")?;
                            writeln!(out, "_args_array[{}].s = _arg_{}_s.as_ptr();", j, j)?;
                            write!(out, "                    ")?;
                            writeln!(out, "_args_array[{}].u = {}.1;", j + 1, a.name)?;
                            write!(out, "                    ")?;
                            writeln!(out, "_args_array[{}].o = ::std::ptr::null_mut();", j + 2)?;
                            j += 2;
                        }
                    }
                    Type::Destructor => panic!("An argument cannot have type \"destructor\"."),
                }
                j += 1;
            }
            writeln!(out, "                    f({}, &mut _args_array)", i)?;
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
        type Request = Request;
        type Event = Event;
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

fn write_client_methods<O: Write>(name: &str, messages: &[Message], out: &mut O) -> IOResult<()> {
    writeln!(out, "    pub trait RequestsTrait {{")?;
    for msg in messages {
        if let Some((ref short, ref long)) = msg.description {
            write_doc(Some(short), long, false, out, 2)?;
        }
        if let Some(Type::Destructor) = msg.typ {
            writeln!(
                out,
                "        ///\n        /// This is a destructor, you cannot send requests to this object any longer once this method is called.",
            )?;
        }
        if msg.since > 1 {
            writeln!(
                out,
                "        ///\n        /// Only available since version {} of the interface",
                msg.since
            )?;
        }
        print_method_prototype(name, &msg, out)?;
        writeln!(out, ";")?;
    }
    writeln!(out, "    }}\n")?;

    writeln!(out, "    impl RequestsTrait for Proxy<{}> {{", name)?;
    for msg in messages {
        let return_type = print_method_prototype(name, &msg, out)?;
        writeln!(out, " {{")?;
        // liveness sanity check
        writeln!(out, "            if !self.is_external() && !self.is_alive() {{")?;
        if return_type.is_some() {
            writeln!(out, "                return Err(());")?;
        } else {
            writeln!(out, "                return;")?;
        }
        writeln!(out, "            }}")?;
        // prepare the proxies if applicable
        let mut has_newp = false;
        for a in &msg.args {
            if a.typ == Type::NewId {
                if let Some(ref iface) = a.interface {
                    writeln!(
                        out,
                        "            let _arg_{}_newproxy = self.child::<super::{}::{}>();",
                        a.name,
                        iface,
                        snake_to_camel(&iface)
                    )?;
                    has_newp = true;
                }
            }
        }
        // actually send the stuff
        write!(
            out,
            "            let msg = Request::{}",
            snake_to_camel(&msg.name)
        )?;
        if msg.args.len() > 0 {
            writeln!(out, " {{")?;
            for a in &msg.args {
                write!(out, "                ")?;
                if a.typ == Type::NewId {
                    if let Some(ref iface) = a.interface {
                        writeln!(
                            out,
                            "{}: unsafe {{ Proxy::<super::{1}::{2}>::from_c_ptr(_arg_{0}_newproxy.c_ptr()) }},",
                            a.name,
                            iface,
                            snake_to_camel(&iface)
                        )?;
                    } else {
                        writeln!(out, "{}: (T::NAME.into(), version, unsafe {{ Proxy::<AnonymousObject>::new_null() }}),", a.name)?;
                    }
                } else if a.typ == Type::Object {
                    if a.allow_null {
                        writeln!(out, "{0} : {0}.map(|o| o.clone()),", a.name)?;
                    } else {
                        writeln!(out, "{0}: {0}.clone(),", a.name)?;
                    }
                } else {
                    writeln!(out, "{0}: {0},", a.name)?;
                }
            }
            write!(out, "            }}")?;
        }
        writeln!(out, ";")?;
        match return_type {
            Some(ret_type) if ret_type.interface.is_none() => {
                writeln!(
                    out,
                    r#"
            unsafe {{
                let ret = msg.as_raw_c_in(|opcode, args| {{
                    ffi_dispatch!(
                        WAYLAND_CLIENT_HANDLE,
                        wl_proxy_marshal_array_constructor_versioned,
                        self.c_ptr(),
                        opcode,
                        args.as_mut_ptr(),
                        T::c_interface(),
                        version
                    )
                }});
                Ok(NewProxy::<T>::from_c_ptr(ret))
            }}"#
                )?;
            }
            _ => {
                writeln!(out, "            self.send(msg);")?;
                if has_newp {
                    for a in &msg.args {
                        if a.typ == Type::NewId {
                            if a.interface.is_some() {
                                writeln!(out, "            Ok(_arg_{}_newproxy)", a.name)?;
                            }
                        }
                    }
                }
            }
        }
        writeln!(out, "        }}\n")?;
    }
    writeln!(out, "    }}")?;

    Ok(())
}

fn print_method_prototype<'a, O: Write>(
    iname: &str,
    msg: &'a Message,
    out: &mut O,
) -> IOResult<Option<&'a Arg>> {
    // detect new_id
    let mut newid = None;
    for arg in &msg.args {
        match arg.typ {
            Type::NewId => if newid.is_some() {
                panic!("Request {}.{} returns more than one new_id", iname, msg.name);
            } else {
                newid = Some(arg);
            },
            _ => (),
        }
    }

    // method start
    match newid {
        Some(arg) if arg.interface.is_none() => {
            write!(
                out,
                "        fn {}{}<T: Interface>(&self, version: u32",
                if is_keyword(&msg.name) { "_" } else { "" },
                msg.name,
            )?;
        }
        _ => {
            write!(
                out,
                "        fn {}{}(&self",
                if is_keyword(&msg.name) { "_" } else { "" },
                msg.name
            )?;
        }
    }

    // print args
    for arg in &msg.args {
        write!(
            out,
            ", {}{}: {}{}{}",
            if is_keyword(&arg.name) { "_" } else { "" },
            arg.name,
            if arg.allow_null { "Option<" } else { "" },
            if let Some(ref name) = arg.enum_ {
                dotted_to_relname(name)
            } else {
                match arg.typ {
                    Type::Object => arg.interface
                        .as_ref()
                        .map(|s| format!("&Proxy<super::{}::{}>", s, snake_to_camel(s)))
                        .unwrap_or(format!("&Proxy<super::AnonymousObject>")),
                    Type::NewId => {
                        // client-side, the return-type handles that
                        continue;
                    }
                    _ => arg.typ.rust_type().into(),
                }
            },
            if arg.allow_null { ">" } else { "" }
        )?;
    }
    write!(out, ") ->")?;

    // return type
    write!(
        out,
        "{}",
        if let Some(arg) = newid {
            arg.interface
                .as_ref()
                .map(|s| format!("Result<NewProxy<super::{}::{}>, ()>", s, snake_to_camel(s)))
                .unwrap_or("Result<NewProxy<T>, ()>".into())
        } else {
            "()".into()
        }
    )?;

    Ok(newid)
}
