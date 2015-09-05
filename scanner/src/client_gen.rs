use std::ascii::AsciiExt;
use std::io::Write;

use protocol::*;

pub fn generate_client_api<O: Write>(protocol: Protocol, out: &mut O) {
    writeln!(out, "//\n// This file was auto-generated, do not edit directly\n//\n").unwrap();

    if let Some(text) = protocol.copyright {
        writeln!(out, "/*\n{}\n*/\n", text).unwrap();
    }

    writeln!(out, "use abi::common::*;").unwrap();
    writeln!(out, "use abi::client::*;").unwrap();
    writeln!(out, "use Proxy;").unwrap();
    writeln!(out, "// update if needed to the appropriate file\nuse super::interfaces::*;\n").unwrap();
    writeln!(out, "use libc::{{c_void, c_char}};\n").unwrap();

    for interface in protocol.interfaces {
        writeln!(out, "//\n// interface {}\n//\n", interface.name).unwrap();

        if let Some((ref summary, ref desc)) = interface.description {
            write_doc(summary, desc, "", out)
        }
        writeln!(out, "pub struct {} {{\n    ptr: *mut c_void\n}}\n",
            snake_to_camel(&interface.name)).unwrap();

        writeln!(out, "impl Proxy for {} {{ fn ptr(&self) -> *mut c_void {{ self.ptr }} }}\n",
            snake_to_camel(&interface.name)).unwrap();

        // emit enums
        for enu in interface.enums {
            if let Some((ref summary, ref desc)) = enu.description {
                write_doc(summary, desc, "", out)
            }
            writeln!(out, "#[repr(i32)]\npub enum {}{} {{",
                snake_to_camel(&interface.name), snake_to_camel(&enu.name)).unwrap();
            if enu.entries.len() == 1 {
                writeln!(out, "    #[doc(hidden)]").unwrap();
                writeln!(out, "    __not_univariant = -1,").unwrap();
            }
            for entry in enu.entries {
                if let Some(summary) = entry.summary {
                    writeln!(out, "    /// {}", summary).unwrap();
                }
                let variantname = snake_to_camel(&entry.name);
                if variantname.chars().next().unwrap().is_digit(10) {
                    writeln!(out, "    {}{} = {},",
                        enu.name.chars().next().unwrap().to_ascii_uppercase(),
                        variantname, entry.value).unwrap();
                } else {
                    writeln!(out, "    {} = {},", variantname, entry.value).unwrap();
                }
            }
            writeln!(out, "}}\n").unwrap();
        }

        // emit opcodes
        let mut i = 0;
        for req in &interface.requests {
            writeln!(out, "const {}_{}: u32 = {};",
                snake_to_screaming(&interface.name), snake_to_screaming(&req.name), i).unwrap();
            i += 1;
        }
        if i > 0 { writeln!(out, "").unwrap() }

        // emit messages
        writeln!(out, "pub enum {}Event {{", snake_to_camel(&interface.name)).unwrap();
        for evt in &interface.events {
            write!(out, "    {}", snake_to_camel(&evt.name)).unwrap();
            if evt.args.len() > 0 {
                write!(out, "(").unwrap();
                for a in &evt.args {
                    write!(out, "{},", a.typ.rust_type()).unwrap();
                }
                write!(out, ")").unwrap();
            }
            writeln!(out, ",").unwrap();
        }
        writeln!(out, "}}\n").unwrap();
    }
    
}

fn write_doc<O: Write>(summary: &str, contents: &str, prefix: &str, out: &mut O) {
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

fn snake_to_camel(input: &str) -> String {
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

fn snake_to_screaming(input: &str) -> String {
    input.chars().map(|c| c.to_ascii_uppercase()).collect()
}
