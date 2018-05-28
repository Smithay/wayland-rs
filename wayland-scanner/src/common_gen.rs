use std::io::Result as IOResult;
use std::io::Write;

use protocol::*;
use util::*;
use Side;

pub(crate) fn write_prefix<O: Write>(protocol: &Protocol, out: &mut O) -> IOResult<()> {
    writeln!(
        out,
        r#"
//
// This file was auto-generated, do not edit directly.
//
"#
    )?;
    if let Some(ref text) = protocol.copyright {
        writeln!(out, "/*\n{}\n*/\n", text)?;
    }
    Ok(())
}

pub(crate) fn write_interface<O: Write, F: FnOnce(&mut O) -> IOResult<()>>(
    name: &str,
    low_name: &str,
    version: u32,
    out: &mut O,
    addon: Option<F>,
) -> IOResult<()> {
    writeln!(
        out,
        r#"
    pub struct {name};

    impl Interface for {name} {{
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "{low_name}";
        const VERSION: u32 = {version};
"#,
        name = name,
        low_name = low_name,
        version = version,
    )?;
    if let Some(addon) = addon {
        addon(out)?;
    }
    writeln!(out, "    }}")?;
    Ok(())
}

pub(crate) fn write_messagegroup<O: Write, F: FnOnce(&mut O) -> IOResult<()>>(
    name: &str,
    side: Side,
    receiver: bool,
    messages: &[Message],
    out: &mut O,
    addon: Option<F>,
) -> IOResult<()> {
    /*
     * Enum definition
     */

    writeln!(out, "    pub enum {} {{", name)?;
    for m in messages {
        if let Some((ref short, ref long)) = m.description {
            write_doc(Some(short), long, false, out, 2)?;
        }
        if let Some(Type::Destructor) = m.typ {
            writeln!(
                out,
                "        ///\n        /// This is a destructor, once {} this object cannot be used any longer.",
                if receiver { "received" } else { "sent" }
            )?;
        }
        if m.since > 1 {
            writeln!(
                out,
                "        ///\n        /// Only available since version {} of the interface",
                m.since
            )?;
        }

        write!(out, "        {}", snake_to_camel(&m.name))?;
        if m.args.len() > 0 {
            write!(out, " {{")?;
            for a in &m.args {
                write!(out, "{}: ", a.name)?;
                if a.allow_null {
                    write!(out, "Option<")?;
                }
                if let Some(ref enu) = a.enum_ {
                    write!(out, "{}", dotted_to_relname(enu))?;
                } else {
                    match a.typ {
                        Type::Uint => write!(out, "u32")?,
                        Type::Int => write!(out, "i32")?,
                        Type::Fixed => write!(out, "f64")?,
                        Type::String => write!(out, "String")?,
                        Type::Array => write!(out, "Vec<u8>")?,
                        Type::Fd => write!(out, "::std::os::unix::io::RawFd")?,
                        Type::Object => {
                            if let Some(ref iface) = a.interface {
                                write!(
                                    out,
                                    "{}<super::{}::{}>",
                                    side.object_name(),
                                    iface,
                                    snake_to_camel(iface)
                                )?;
                            } else {
                                write!(out, "{}<AnonymousObject>", side.object_name())?;
                            }
                        }
                        Type::NewId => {
                            if let Some(ref iface) = a.interface {
                                write!(
                                    out,
                                    "{}{}<super::{}::{}>",
                                    if receiver { "New" } else { "" },
                                    side.object_name(),
                                    iface,
                                    snake_to_camel(iface)
                                )?;
                            } else {
                                // bind-like function
                                write!(
                                    out,
                                    "(String, u32, {}{}<AnonymousObject>)",
                                    if receiver { "New" } else { "" },
                                    side.object_name()
                                )?;
                            }
                        }
                        Type::Destructor => panic!("An argument cannot have type \"destructor\"."),
                    }
                }
                if a.allow_null {
                    write!(out, ">")?;
                }
                write!(out, ", ")?;
            }
            write!(out, "}}")?;
        }
        writeln!(out, ",")?
    }
    writeln!(out, "    }}\n")?;

    /*
     * MessageGroup implementation
     */

    writeln!(out, "    impl super::MessageGroup for {} {{", name)?;

    // MESSAGES
    writeln!(out, "        const MESSAGES: &'static [super::MessageDesc] = &[")?;
    for msg in messages {
        writeln!(out, "            super::MessageDesc {{")?;
        writeln!(out, "                name: \"{}\",", msg.name)?;
        writeln!(out, "                since: {},", msg.since)?;
        writeln!(out, "                signature: &[")?;
        for arg in &msg.args {
            writeln!(
                out,
                "                    super::ArgumentType::{},",
                arg.typ.common_type()
            )?;
        }
        writeln!(out, "                ]")?;
        writeln!(out, "            }},")?;
    }
    writeln!(out, "        ];")?;

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

    if let Some(addon) = addon {
        addon(out)?;
    }

    writeln!(out, "    }}\n")?;

    Ok(())
}

pub(crate) fn write_enums<O: Write>(enums: &[Enum], out: &mut O) -> IOResult<()> {
    // generate contents
    for enu in enums {
        if enu.bitfield {
            writeln!(out, "    bitflags! {{")?;
            if let Some((ref short, ref long)) = enu.description {
                write_doc(Some(short), long, false, out, 2)?;
            }
            writeln!(out, "        pub struct {}: u32 {{", snake_to_camel(&enu.name))?;
            for entry in &enu.entries {
                if let Some((ref short, ref long)) = entry.description {
                    write_doc(Some(short), long, false, out, 3)?;
                } else if let Some(ref summary) = entry.summary {
                    writeln!(out, "            /// {}", summary)?;
                }
                writeln!(
                    out,
                    "            const {}{} = {};",
                    if entry.name.chars().next().unwrap().is_numeric() {
                        "_"
                    } else {
                        ""
                    },
                    snake_to_camel(&entry.name),
                    entry.value
                )?;
            }
            writeln!(out, "        }}")?;
            writeln!(out, "    }}")?;
            writeln!(out, "    impl {} {{", snake_to_camel(&enu.name))?;
            writeln!(
                out,
                "        pub fn from_raw(n: u32) -> Option<{}> {{",
                snake_to_camel(&enu.name)
            )?;
            writeln!(
                out,
                "            Some({}::from_bits_truncate(n))",
                snake_to_camel(&enu.name)
            )?;
            writeln!(
                out,
                r#"
        }}
        pub fn to_raw(&self) -> u32 {{
            self.bits()
        }}
    }}
"#
            )?;
        } else {
            // if enu.bitfield
            if let Some((ref short, ref long)) = enu.description {
                write_doc(Some(short), long, false, out, 1)?;
            }
            writeln!(
                out,
                r#"
    #[repr(u32)]
    #[derive(Copy,Clone,Debug,PartialEq)]
    pub enum {} {{"#,
                snake_to_camel(&enu.name)
            )?;
            for entry in &enu.entries {
                if let Some((ref short, ref long)) = entry.description {
                    write_doc(Some(short), long, false, out, 2)?;
                } else if let Some(ref summary) = entry.summary {
                    writeln!(out, "        /// {}", summary)?;
                }
                writeln!(
                    out,
                    "        {}{} = {},",
                    if entry.name.chars().next().unwrap().is_numeric() {
                        "_"
                    } else {
                        ""
                    },
                    snake_to_camel(&entry.name),
                    entry.value
                )?;
            }
            writeln!(out, "    }}")?;

            writeln!(out, "    impl {} {{", snake_to_camel(&enu.name))?;
            writeln!(
                out,
                "        pub fn from_raw(n: u32) -> Option<{}> {{",
                snake_to_camel(&enu.name)
            )?;
            writeln!(out, "            match n {{")?;
            for entry in &enu.entries {
                writeln!(
                    out,
                    "                {} => Some({}::{}{}),",
                    entry.value,
                    snake_to_camel(&enu.name),
                    if entry.name.chars().next().unwrap().is_numeric() {
                        "_"
                    } else {
                        ""
                    },
                    snake_to_camel(&entry.name)
                )?;
            }
            writeln!(
                out,
                r#"
                _ => Option::None
            }}
        }}
        pub fn to_raw(&self) -> u32 {{
            *self as u32
        }}
    }}
"#
            )?;
        }
    }
    Ok(())
}

pub(crate) fn write_doc<O: Write>(
    short: Option<&str>,
    long: &str,
    internal: bool,
    out: &mut O,
    indent: usize,
) -> IOResult<()> {
    let p = if internal { '!' } else { '/' };
    if let Some(txt) = short {
        for _ in 0..indent {
            write!(out, "    ")?
        }
        writeln!(out, "//{} {}", p, txt)?;
        for _ in 0..indent {
            write!(out, "    ")?
        }
        writeln!(out, "//{}", p)?;
    }
    for l in long.lines() {
        for _ in 0..indent {
            write!(out, "    ")?
        }
        writeln!(out, "//{} {}", p, l.trim())?;
    }
    Ok(())
}

pub fn print_method_prototype<'a, O: Write>(
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
        Some(arg) => if arg.interface.is_none() {
            write!(
                out,
                "        fn {}{}<T: Interface, F>(&self, version: u32",
                if is_keyword(&msg.name) { "_" } else { "" },
                msg.name,
            )?;
        } else {
            write!(
                out,
                "        fn {}{}<F>(&self",
                if is_keyword(&msg.name) { "_" } else { "" },
                msg.name
            )?;
        },
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
    if newid.is_some() {
        write!(out, ", implementor: F")?;
    }
    write!(out, ") ->")?;

    // return type and bound
    if let Some(ref arg) = newid {
        match arg.interface {
            Some(ref iface) => {
                write!(
                    out,
                    "Result<Proxy<super::{module}::{name}>, ()>
                    where F: FnOnce(NewProxy<super::{module}::{name}>) -> Proxy<super::{module}::{name}>",
                    module = iface,
                    name = snake_to_camel(iface)
                )?;
            }
            None => {
                write!(
                    out,
                    "Result<Proxy<T>, ()>
                    where F: FnOnce(NewProxy<T>) -> Proxy<T>"
                )?;
            }
        }
    } else {
        write!(out, "()")?;
    }

    Ok(newid)
}
