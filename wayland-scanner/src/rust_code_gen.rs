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
