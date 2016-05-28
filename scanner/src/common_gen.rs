use std::ascii::AsciiExt;
use std::io::Write;

use protocol::*;

pub fn write_doc<O: Write>(summary: &str, contents: &str, prefix: &str, out: &mut O) {
    writeln!(out, "{}/// {}", prefix, summary).unwrap();
    writeln!(out, "{}///", prefix).unwrap();
    for l in contents.lines() {
        let trimmed = l.trim();
        if trimmed.len() > 0 {
            writeln!(out, "{}/// {}", prefix, trimmed).unwrap();
        } else {
            writeln!(out, "{}///", prefix).unwrap();
        }
    }
}

pub fn snake_to_camel(input: &str) -> String {
    input.split('_').flat_map(|s| {
        let mut first = true;
        s.chars().map(move |c| {
            if first {
                first = false;
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
    }).collect()
}

pub fn dotted_snake_to_camel(input: &str) -> String {
    input.split('_').flat_map(|s| s.split('.')).flat_map(|s| {
        let mut first = true;
        s.chars().map(move |c| {
            if first {
                first = false;
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
    }).collect()
}

pub fn snake_to_screaming(input: &str) -> String {
    input.chars().map(|c| c.to_ascii_uppercase()).collect()
}

pub fn emit_opcodes<O: Write>(interface_name: &str, messages: &[Message], out: &mut O) {
    writeln!(out, "// {} opcodes", interface_name).unwrap();
    let mut i = 0;
    for msg in messages {
        writeln!(out, "const {}_{}: u32 = {};",
            snake_to_screaming(&interface_name), snake_to_screaming(&msg.name), i).unwrap();
        i += 1;
    }
    if i > 0 { writeln!(out, "").unwrap() }

}

pub fn emit_enums<O: Write>(enums: &[Enum], iface_name: &str, out: &mut O) -> Vec<String> {
    let mut bitfields = Vec::new();

    for enu in enums {
        if enu.bitfield {
            bitfields.push(format!("{}.{}", iface_name, enu.name));
        }
        if let Some((ref summary, ref desc)) = enu.description {
            write_doc(summary, desc, "", out)
        }
        if enu.bitfield {
            writeln!(out, "pub mod {}{} {{",
                snake_to_camel(iface_name), snake_to_camel(&enu.name)).unwrap();
            writeln!(out, "bitflags! {{").unwrap();
            writeln!(out, "    pub flags {}{}: u32 {{",
                snake_to_camel(iface_name), snake_to_camel(&enu.name)).unwrap();
        } else {
            writeln!(out, "#[repr(u32)]\n#[derive(Debug)]\npub enum {}{} {{",
                snake_to_camel(iface_name), snake_to_camel(&enu.name)).unwrap();
        }
        for entry in &enu.entries {
            if let Some(ref summary) = entry.summary {
                writeln!(out, "    /// {}", summary).unwrap();
            }
            let variantname = snake_to_camel(&entry.name);
            write!(out, "    ").unwrap();
            if enu.bitfield {
                write!(out, "    const ").unwrap();
            }
            if variantname.chars().next().unwrap().is_digit(10) {
                writeln!(out, "{}{} = {},",
                    enu.name.chars().next().unwrap().to_ascii_uppercase(),
                    variantname, entry.value).unwrap();
            } else {
                writeln!(out, "{} = {},", variantname, entry.value).unwrap();
            }
        }
        if enu.bitfield {
            writeln!(out, "    }}").unwrap();
        } else if enu.entries.len() == 1 {
            writeln!(out, "    #[doc(hidden)]").unwrap();
            writeln!(out, "    __not_univariant,").unwrap();
        }
        writeln!(out, "}}").unwrap();

        if enu.bitfield {
            writeln!(out, "}}\n").unwrap();
        }

        if enu.bitfield {
            writeln!(out, "fn {}_{}_from_raw(n: u32) -> Option<{}{}::{}{}> {{",
                iface_name, &enu.name,
                snake_to_camel(iface_name), snake_to_camel(&enu.name),
                snake_to_camel(iface_name), snake_to_camel(&enu.name)).unwrap();
            writeln!(out, "    Some({}{}::{}{}::from_bits_truncate(n))",
                snake_to_camel(iface_name), snake_to_camel(&enu.name),
                snake_to_camel(iface_name), snake_to_camel(&enu.name)).unwrap();
            writeln!(out, "}}\n").unwrap();
        } else {
            writeln!(out, "fn {}_{}_from_raw(n: u32) -> Option<{}{}> {{",
                iface_name, &enu.name,
                snake_to_camel(iface_name), snake_to_camel(&enu.name)).unwrap();
            writeln!(out, "    match n {{").unwrap();
            for entry in &enu.entries {
                let variantname = snake_to_camel(&entry.name);
                if variantname.chars().next().unwrap().is_digit(10) {
                    writeln!(out, "        {} => Some({}{}::{}{}),",
                        entry.value,
                        snake_to_camel(iface_name), snake_to_camel(&enu.name),
                        enu.name.chars().next().unwrap().to_ascii_uppercase(),
                        variantname).unwrap();
                } else {
                    writeln!(out, "        {} => Some({}{}::{}),",
                        entry.value,
                        snake_to_camel(iface_name), snake_to_camel(&enu.name),
                        variantname).unwrap();
                }
            }
            writeln!(out, "        _ => None").unwrap();
            writeln!(out, "    }}").unwrap();
            writeln!(out, "}}\n").unwrap();
        }
    }

    return bitfields;
}

pub fn emit_message_enums<O: Write>(messages: &[Message], iname: &str, camel_iname: &str, bitfields: &[String], server: bool, out: &mut O) {
    writeln!(out, "#[derive(Debug)]").unwrap();
    writeln!(out, "pub enum {}{} {{", camel_iname, if server { "Request" } else { "Event" }).unwrap();
    for msg in messages {
        if let Some((ref summary, ref desc)) = msg.description {
            write_doc(summary, desc, "    ", out)
        }
        if msg.args.len() > 0 {
            write!(out, "    ///\n    /// Values:").unwrap();
            for a in &msg.args {
                write!(out, " {},", a.name).unwrap();
            }
            writeln!(out, "").unwrap();
        }
        write!(out, "    {}", snake_to_camel(&msg.name)).unwrap();
        if msg.args.len() > 0 {
            write!(out, "(").unwrap();
            for a in &msg.args {
                if let Some(ref enu) = a.enum_ {
                    if enu.contains('.'){
                        // this is an enum from an other iface
                        if bitfields.contains(enu) {
                        write!(out, "{}::",
                                dotted_snake_to_camel(enu)).unwrap();
                        }
                        write!(out, "{},",
                            dotted_snake_to_camel(enu),
                        ).unwrap();
                    } else {
                        if bitfields.contains(&format!("{}.{}", iname, enu)) {
                        write!(out, "{}{}::",
                                camel_iname, snake_to_camel(enu)).unwrap();
                        }
                        write!(out, "{}{},",
                            camel_iname, snake_to_camel(enu),
                        ).unwrap();
                    }
                } else if a.typ == Type::NewId {
                    write!(out, "{},",
                        a.interface.as_ref().map(|s| snake_to_camel(s))
                            .expect("Cannot create a new_id in an event without an interface.")
                    ).unwrap();
                } else if a.typ == Type::Object {
                    if server {
                        write!(out, "ResourceId,").unwrap();
                    } else {
                        write!(out, "ProxyId,").unwrap();
                    }
                } else {
                    write!(out, "{},", a.typ.rust_type()).unwrap();
                }
            }
            write!(out, ")").unwrap();
        }
        writeln!(out, ",").unwrap();
    }
    writeln!(out, "}}\n").unwrap();
}
