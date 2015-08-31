use std::ascii::AsciiExt;

use protocol::*;

pub fn generate_client_api(protocol: Protocol) {
    println!("//\n// This file was auto-generated, do not edit directly\n//\n");

    if let Some(text) = protocol.copyright {
        println!("/*\n{}\n*/\n", text);
    }

    println!("#![allow(dead_code,non_camel_case_types)]\n");

    println!("use abi::common::*;");
    println!("use abi::client::*;");
    println!("use Proxy;");
    println!("// update if needed to the appropriate file\nuse super::interfaces::*;\n");
    println!("use libc::{{c_void, c_char}};\n");

    for interface in protocol.interfaces {
        println!("//\n// interface {}\n//\n", interface.name);

        if let Some((ref summary, ref desc)) = interface.description {
            write_doc(summary, desc, "")
        }
        println!("pub struct {} {{\n    ptr: *mut c_void\n}}\n", snake_to_camel(&interface.name));

        println!("impl Proxy for {} {{ fn ptr(&self) -> *mut c_void {{ self.ptr }} }}\n",
            snake_to_camel(&interface.name));

        // emit enums
        for enu in interface.enums {
            if let Some((ref summary, ref desc)) = enu.description {
                write_doc(summary, desc, "")
            }
            println!("#[repr(i32)]\npub enum {}{} {{", snake_to_camel(&interface.name), snake_to_camel(&enu.name));
            if enu.entries.len() == 1 {
                println!("    #[doc(hidden)]");
                println!("    __not_univariant = -1,");
            }
            for entry in enu.entries {
                if let Some(summary) = entry.summary {
                    println!("    /// {}", summary);
                }
                let variantname = snake_to_camel(&entry.name);
                if variantname.chars().next().unwrap().is_digit(10) {
                    println!("    {}{} = {},",
                        enu.name.chars().next().unwrap().to_ascii_uppercase(), variantname, entry.value);
                } else {
                    println!("    {} = {},", variantname, entry.value);
                }
            }
            println!("}}");
        }
    }
    
}

fn write_doc(summary: &str, contents: &str, prefix: &str) {
    println!("{}/// {}", prefix, summary);
    println!("{}///", prefix);
    for l in contents.lines() {
        let trimmed = l.trim();
        if trimmed.len() > 0 {
            println!("{}/// {}", prefix, trimmed);
        } else {
            println!("{}///", prefix);
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