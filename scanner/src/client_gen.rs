use std::ascii::AsciiExt;
use std::borrow::Cow;
use std::io::Write;

use protocol::*;

pub fn generate_client_api<O: Write>(protocol: Protocol, out: &mut O) {
    writeln!(out, "//\n// This file was auto-generated, do not edit directly\n//\n").unwrap();

    if let Some(text) = protocol.copyright {
        writeln!(out, "/*\n{}\n*/\n", text).unwrap();
    }

    writeln!(out, "use abi::common::*;").unwrap();
    writeln!(out, "use abi::client::*;").unwrap();
    writeln!(out, "use {{Proxy, ProxyId}};").unwrap();
    writeln!(out, "// update if needed to the appropriate file\nuse super::interfaces::*;\n").unwrap();
    writeln!(out, "use std::ffi::CString;").unwrap();
    writeln!(out, "use std::ptr;").unwrap();

    for interface in protocol.interfaces {
        let camel_iname = snake_to_camel(&interface.name);
        writeln!(out, "//\n// interface {}\n//\n", interface.name).unwrap();

        if let Some((ref summary, ref desc)) = interface.description {
            write_doc(summary, desc, "", out)
        }
        writeln!(out, "pub struct {} {{\n    ptr: *mut wl_proxy\n}}\n",
            camel_iname).unwrap();

        // Id
        writeln!(out, "#[derive(PartialEq,Eq,Copy,Clone)]").unwrap();
        writeln!(out, "pub struct {}Id {{ id: usize }}\n", camel_iname).unwrap();
        writeln!(out, "impl From<{}Id> for ProxyId {{ fn from(other: {}Id) -> ProxyId {{ ProxyId {{ id: other.id }} }} }}", camel_iname, camel_iname).unwrap();

        writeln!(out, "impl Proxy for {} {{", camel_iname).unwrap();
        writeln!(out, "    type Id = {}Id;", camel_iname).unwrap();
        writeln!(out, "    fn ptr(&self) -> *mut wl_proxy {{ self.ptr }}").unwrap();
        writeln!(out, "    fn interface() -> *mut wl_interface {{ unsafe {{ &mut {}_interface  as *mut wl_interface }} }}", interface.name).unwrap();
        writeln!(out, "    fn id(&self) -> {}Id {{ {}Id {{ id: self.ptr as usize }} }}", camel_iname, camel_iname).unwrap();
        writeln!(out, "    unsafe fn from_ptr(ptr: *mut wl_proxy) -> {} {{ {} {{ ptr: ptr }} }}", camel_iname, camel_iname).unwrap();
        writeln!(out, "}}").unwrap();
            

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
        writeln!(out, "// {} opcodes", interface.name).unwrap();
        let mut i = 0;
        for req in &interface.requests {
            writeln!(out, "const {}_{}: u32 = {};",
                snake_to_screaming(&interface.name), snake_to_screaming(&req.name), i).unwrap();
            i += 1;
        }
        if i > 0 { writeln!(out, "").unwrap() }

        // emit messages
        if interface.events.len() > 0 {
            writeln!(out, "pub enum {}Event {{", camel_iname).unwrap();
            for evt in &interface.events {
                if let Some((ref summary, ref desc)) = evt.description {
                    write_doc(summary, desc, "    ", out)
                }
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

        // impl
        writeln!(out, "impl {} {{", camel_iname).unwrap();
        // requests
        for req in &interface.requests {
            if req.typ == Some(Type::Destructor) {
                // TODO
                continue;
            }
            let new_id_interfaces: Vec<Option<String>> = req.args.iter()
                .filter(|a| a.typ == Type::NewId)
                .map(|a| a.interface.clone())
                .collect();
            if new_id_interfaces.len() > 1 {
                // TODO: can we handle this properly ?
                continue;
            }
            let ret = new_id_interfaces.into_iter().next();

            writeln!(out, "").unwrap();

            if let Some((ref summary, ref doc)) = req.description {
                write_doc(summary, doc, "    ", out);
            }
            write!(out, "    pub fn {}{}", req.name, if ::util::is_keyword(&req.name) { "_" } else { "" }).unwrap();
            if let Some(ref newint) = ret {
                if newint.is_none() {
                    write!(out, "<T: Proxy>").unwrap();
                }
            }
            write!(out, "(&self,").unwrap();
            for a in &req.args {
                if a.typ == Type::NewId { continue; }
                let typ: Cow<str> = if a.typ == Type::Object {
                    a.interface.as_ref().map(|i| format!("&{}", snake_to_camel(i)).into()).unwrap_or("*mut ()".into())
                } else {
                    a.typ.rust_type().into()
                };
                if a.allow_null {
                    write!(out, " {}: Option<{}>,", a.name, typ).unwrap();
                } else {
                    write!(out, " {}: {},", a.name, typ).unwrap();
                }
            }
            if let Some(ref newint) = ret {
                if newint.is_none() {
                    write!(out, "version: u32,").unwrap();
                }
            }
            write!(out, ")").unwrap();
            if let Some(ref newint) = ret {
                write!(out, " -> {}", newint.as_ref().map(|t| snake_to_camel(t)).unwrap_or("T".to_owned())).unwrap();
            }
            writeln!(out, " {{").unwrap();
            // function body
            for a in &req.args {
                match a.typ {
                    Type::String => {
                        if a.allow_null {
                            writeln!(out, "        let {} = {}.map(|t| CString::new(t).expect(\"Got a String with interior null.\"));",
                                a.name, a.name).unwrap();
                        } else {
                            writeln!(out, "        let {} = CString::new({}).expect(\"Got a String with interior null.\");",
                                a.name, a.name).unwrap();
                        }
                    },
                    Type::Fixed => {
                        writeln!(out, "        let {} = wl_fixed_from_double({})", a.name, a.name).unwrap();
                    },
                    _ => {}
                }
            }
            if let Some(ref newint) = ret {
                if let &Some(ref name) = newint {
                    writeln!(out, "        let ptr = unsafe {{ wl_proxy_marshal_constructor(self.ptr(), {}_{}, &{}_interface as *const wl_interface,",
                        snake_to_screaming(&interface.name), snake_to_screaming(&req.name), name).unwrap();
                } else {
                    writeln!(out, "        let ptr = unsafe {{ wl_proxy_marshal_constructor(self.ptr(), {}_{}, <T as Proxy>::interface(),",
                        snake_to_screaming(&interface.name), snake_to_screaming(&req.name)).unwrap();
                }
            } else {
                writeln!(out, "        unsafe {{ wl_proxy_marshal(self.ptr(), {}_{},",
                    snake_to_screaming(&interface.name), snake_to_screaming(&req.name)).unwrap();
            }
            write!(out, "           ").unwrap();
            for a in &req.args {
                if a.typ == Type::NewId {
                    if let Some(ref newint) = ret {
                        if newint.is_none() {
                            write!(out, "(*<T as Proxy>::interface()).name, version, ").unwrap();
                        }
                    }
                    write!(out, "ptr::null_mut::<wl_proxy>(),").unwrap();
                } else if a.typ == Type::String {
                    if a.allow_null {
                        write!(out, " {}.map(|s| s.as_ptr()).unwrap_or(ptr::null()),", a.name).unwrap();
                    } else {
                        write!(out, " {}.as_ptr(),", a.name).unwrap();
                    }
                } else if a.typ == Type::Array {
                    if a.allow_null {
                        write!(out, "{}.map(|a| &mut a as *mut wl_array).unwrap_or(ptr::null_mut())", a.name).unwrap();
                    } else {
                        write!(out, " &mut {} as *mut wl_array,", a.name).unwrap();
                    }
                } else {
                    write!(out, " {},", a.name).unwrap();
                }
            }
            writeln!(out, ") }};").unwrap();
            if ret.is_some() {
                writeln!(out, "        unsafe {{ Proxy::from_ptr(ptr) }}").unwrap();
            }
            writeln!(out, "    }}").unwrap();
        }

        // TODO destroy/destructor

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
