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

pub(crate) fn write_messagegroup<O: Write>(
    name: &str,
    side: Side,
    receiver: bool,
    messages: &[Message],
    out: &mut O,
) -> IOResult<()> {
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
