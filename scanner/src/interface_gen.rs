use std::cmp;

use protocol::*;

macro_rules! for_requests_and_events_of_interface(
    ($interface: expr, $name: ident, $code:expr) => (
        for $name in &mut $interface.requests {
            $code
        }
        for $name in &mut $interface.events {
            $code
        }
    );
);

pub fn generate_interfaces(protocol: Protocol) {
    println!("//\n// This file was auto-generated, do not edit directly\n//\n");

    if let Some(text) = protocol.copyright {
        println!("/*\n{}\n*/\n", text);
    }

    println!("#![allow(dead_code,non_camel_case_types)]\n");

    println!("use abi::common::*;\n");
    println!("use libc::{{c_void, c_char}};\n");

    //
    // null types array
    //

    let longest_nulls = protocol.interfaces.iter().fold(0, |max, interface| {
        let request_longest_null = interface.requests.iter().fold(0, |max, request| {
            if request.all_null() { cmp::max(request.args.len(), max) } else { max }
        });
        let events_longest_null = interface.events.iter().fold(0, |max, event| {
            if event.all_null() { cmp::max(event.args.len(), max) } else { max }
        });
        cmp::max(max, cmp::max(request_longest_null, events_longest_null))
    });
    
    println!("const NULLPTR : *const c_void = 0 as *const c_void;\n");

    println!("static mut types_null: [*const wl_interface; {}] = [", longest_nulls);
    for _ in 0..longest_nulls {
        println!("    NULLPTR as *const wl_interface,");
    }
    println!("];\n");

    //
    // emit interfaces
    //

    macro_rules! emit_messages(
        ($interface: expr, $which: ident) => (
        if $interface.$which.len() != 0 {
            // first, emit types arrays for the messages
            for msg in &$interface.$which {
                if msg.all_null() { continue; }
                println!("static mut {}_{}_{}_types: [*const wl_interface; {}] = [",
                    $interface.name, stringify!($which), msg.name, msg.args.len());
                for arg in &msg.args {
                    match (arg.typ, &arg.interface) {
                        (Type::Object, &Some(ref inter)) | (Type::NewId, &Some(ref inter)) => {
                           println!("    unsafe {{ &{}_interface as *const wl_interface }},", inter)
                        }
                        _ => println!("    NULLPTR as *const wl_interface,")
                    }
                }
                println!("];");
            }

            // then, the message array
            println!("pub static mut {}_{}: [wl_message; {}] = [",
                $interface.name, stringify!($which), $interface.$which.len());
            for msg in &$interface.$which {
                print!("    wl_message {{ name: b\"{}\" as *const u8 as *const c_char, signature: b\"", msg.name);
                if msg.since > 1 { print!("{}", msg.since); }
                for arg in &msg.args {
                    if arg.typ.nullable() && arg.allow_null { print!("?"); }
                    match arg.typ {
                        Type::NewId => {
                            if arg.interface.is_none() { print!("su"); }
                            print!("n");
                        },
                        Type::Uint => print!("u"),
                        Type::Fixed => print!("f"),
                        Type::String => print!("s"),
                        Type::Object => print!("o"),
                        Type::Array => print!("a"),
                        Type::Fd => print!("h"),
                        Type::Int => print!("i"),
                        _ => {}
                    }
                }
                print!("\" as *const u8 as *const c_char, types: ");
                if msg.all_null() {
                    print!("unsafe {{ &types_null as *const _ }}");
                } else {
                    print!("unsafe {{ &{}_{}_{}_types as *const _ }}", $interface.name, stringify!($which), msg.name);
                }
                println!(" }},");
            }
            println!("];");
        });
    );

    for interface in &protocol.interfaces {

        println!("// {}\n", interface.name);

        emit_messages!(interface, requests);
        emit_messages!(interface, events);

        println!("\nstatic mut {}_interface: wl_interface = wl_interface {{", interface.name);
        println!("    name: b\"{}\" as *const u8  as *const c_char,", interface.name);
        println!("    version: {},", interface.version);
        println!("    request_count: {},", interface.requests.len());
        if interface.requests.len() > 0 {
            println!("    requests: unsafe {{ &{}_requests as *const _ }},", interface.name);
        } else {
            println!("    requests: NULLPTR as *const wl_message,");
        }
        println!("    event_count: {},", interface.events.len());
        if interface.events.len() > 0 {
            println!("    events: unsafe {{ &{}_events as *const _ }},", interface.name);
        } else {
            println!("    events: NULLPTR as *const wl_message,");
        }
        println!("}};");

        println!("");
    }
}