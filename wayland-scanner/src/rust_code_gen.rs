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
            "    use super::{{Proxy, NewProxy, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType, Object, Message, Argument, ObjectMetadata}};\n"
        )?;
        let iface_name = snake_to_camel(&iface.name);

        write_enums(&iface.enums, out)?;
        write_messagegroup(
            "Request",
            Side::Client,
            false,
            &iface.requests,
            out,
            None::<fn(_: &mut _) -> _>,
        )?;
        write_messagegroup(
            "Event",
            Side::Client,
            true,
            &iface.events,
            out,
            None::<fn(_: &mut _) -> _>,
        )?;
        write_interface(
            &iface_name,
            &iface.name,
            iface.version,
            out,
            None::<fn(_: &mut _) -> _>,
        )?;
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
            "    use super::{{Resource, NewResource, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType, Object, Message, Argument, ObjectMetadata}};\n"
        )?;
        let iface_name = snake_to_camel(&iface.name);

        write_enums(&iface.enums, out)?;
        write_messagegroup(
            "Request",
            Side::Server,
            true,
            &iface.requests,
            out,
            None::<fn(_: &mut _) -> _>,
        )?;
        write_messagegroup(
            "Event",
            Side::Server,
            false,
            &iface.events,
            out,
            None::<fn(_: &mut _) -> _>,
        )?;
        write_interface(
            &iface_name,
            &iface.name,
            iface.version,
            out,
            None::<fn(_: &mut _) -> _>,
        )?;

        writeln!(out, "}}\n")?;
    }

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
        writeln!(out, "            if !self.is_alive() {{")?;
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
                if a.interface.is_some() {
                    writeln!(
                        out,
                        "            let _arg_{}_newproxy = implementor(self.child());",
                        a.name,
                    )?;
                } else {
                    writeln!(
                        out,
                        "            let _arg_{}_newproxy = implementor(self.child_versioned(version));",
                        a.name,
                    )?;
                }
                has_newp = true;
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
                    if a.interface.is_some() {
                        writeln!(out, "{}: _arg_{0}_newproxy.clone(),", a.name,)?;
                    } else {
                        writeln!(
                            out,
                            "{}: (T::NAME.into(), version, _arg_{0}_newproxy.anonymize()),",
                            a.name
                        )?;
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
        writeln!(out, "            self.send(msg);")?;
        if has_newp {
            for a in &msg.args {
                if a.typ == Type::NewId {
                    writeln!(out, "            Ok(_arg_{}_newproxy)", a.name)?;
                }
            }
        }
        writeln!(out, "        }}\n")?;
    }
    writeln!(out, "    }}")?;

    Ok(())
}
