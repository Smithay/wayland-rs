

use protocol::*;
use std::cmp;
use std::io::Write;

pub fn generate_interfaces<O: Write>(protocol: Protocol, out: &mut O) {
    writeln!(
        out,
        "//\n// This file was auto-generated, do not edit directly\n//\n"
    ).unwrap();

    if let Some(text) = protocol.copyright {
        writeln!(out, "/*\n{}\n*/\n", text).unwrap();
    }

    writeln!(out, "use std::os::raw::{{c_char, c_void}};\n").unwrap();
    writeln!(out, "use wayland_sys::common::*;\n").unwrap();

    // null types array
    //

    let longest_nulls = protocol.interfaces.iter().fold(0, |max, interface| {
        let request_longest_null = interface.requests.iter().fold(0, |max, request| {
            if request.all_null() {
                cmp::max(request.args.len(), max)
            } else {
                max
            }
        });
        let events_longest_null = interface.events.iter().fold(
            0,
            |max, event| if event.all_null() {
                cmp::max(event.args.len(), max)
            } else {
                max
            },
        );
        cmp::max(max, cmp::max(request_longest_null, events_longest_null))
    });

    writeln!(out, "const NULLPTR: *const c_void = 0 as *const c_void;\n").unwrap();

    writeln!(
        out,
        "static mut types_null: [*const wl_interface; {}] = [",
        longest_nulls
    ).unwrap();
    for _ in 0..longest_nulls {
        writeln!(out, "    NULLPTR as *const wl_interface,").unwrap();
    }
    writeln!(out, "];\n").unwrap();

    // emit interfaces
    //

    macro_rules! emit_messages(
        ($interface: expr, $which: ident) => (
        if $interface.$which.len() != 0 {
            // first, emit types arrays for the messages
            for msg in &$interface.$which {
                if msg.all_null() { continue; }
                writeln!(out, "static mut {}_{}_{}_types: [*const wl_interface; {}] = [",
                    $interface.name, stringify!($which), msg.name, msg.args.len()).unwrap();
                for arg in &msg.args {
                    match (arg.typ, &arg.interface) {
                        (Type::Object, &Some(ref inter)) | (Type::NewId, &Some(ref inter)) => {
                           writeln!(out, "    unsafe {{ &{}_interface as *const wl_interface }},", inter).unwrap()
                        }
                        _ => writeln!(out, "    NULLPTR as *const wl_interface,").unwrap()
                    }
                }
                writeln!(out, "];").unwrap();
            }

            // then, the message array
            writeln!(out, "pub static mut {}_{}: [wl_message; {}] = [",
                $interface.name, stringify!($which), $interface.$which.len()).unwrap();
            for msg in &$interface.$which {
                write!(out, "    wl_message {{ name: b\"{}\\0\" as *const u8 as *const c_char, signature: b\"",
                    msg.name).unwrap();
                if msg.since > 1 { write!(out, "{}", msg.since).unwrap(); }
                for arg in &msg.args {
                    if arg.typ.nullable() && arg.allow_null { write!(out, "?").unwrap(); }
                    match arg.typ {
                        Type::NewId => {
                            if arg.interface.is_none() { write!(out, "su").unwrap(); }
                            write!(out, "n").unwrap();
                        },
                        Type::Uint => write!(out, "u").unwrap(),
                        Type::Fixed => write!(out, "f").unwrap(),
                        Type::String => write!(out, "s").unwrap(),
                        Type::Object => write!(out, "o").unwrap(),
                        Type::Array => write!(out, "a").unwrap(),
                        Type::Fd => write!(out, "h").unwrap(),
                        Type::Int => write!(out, "i").unwrap(),
                        _ => {}
                    }
                }
                write!(out, "\\0\" as *const u8 as *const c_char, types: ").unwrap();
                if msg.all_null() {
                    write!(out, "unsafe {{ &types_null as *const _ }}").unwrap();
                } else {
                    write!(out, "unsafe {{ &{}_{}_{}_types as *const _ }}",
                        $interface.name, stringify!($which), msg.name).unwrap();
                }
                writeln!(out, " }},").unwrap();
            }
            writeln!(out, "];").unwrap();
        });
    );

    for interface in &protocol.interfaces {

        writeln!(out, "// {}\n", interface.name).unwrap();

        emit_messages!(interface, requests);
        emit_messages!(interface, events);

        writeln!(
            out,
            "\npub static mut {}_interface: wl_interface = wl_interface {{",
            interface.name
        ).unwrap();
        writeln!(
            out,
            "    name: b\"{}\\0\" as *const u8 as *const c_char,",
            interface.name
        ).unwrap();
        writeln!(out, "    version: {},", interface.version).unwrap();
        writeln!(out, "    request_count: {},", interface.requests.len()).unwrap();
        if interface.requests.len() > 0 {
            writeln!(
                out,
                "    requests: unsafe {{ &{}_requests as *const _ }},",
                interface.name
            ).unwrap();
        } else {
            writeln!(out, "    requests: NULLPTR as *const wl_message,").unwrap();
        }
        writeln!(out, "    event_count: {},", interface.events.len()).unwrap();
        if interface.events.len() > 0 {
            writeln!(
                out,
                "    events: unsafe {{ &{}_events as *const _ }},",
                interface.name
            ).unwrap();
        } else {
            writeln!(out, "    events: NULLPTR as *const wl_message,").unwrap();
        }
        writeln!(out, "}};").unwrap();

        writeln!(out, "").unwrap();
    }
}
