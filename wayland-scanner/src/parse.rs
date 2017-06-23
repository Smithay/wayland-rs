

use protocol::*;
use std::io::Read;

use xml::EventReader;
use xml::attribute::OwnedAttribute;
use xml::reader::Events;
use xml::reader::ParserConfig;
use xml::reader::XmlEvent;

macro_rules! extract_from(
    ($it: expr => $pattern: pat => $result: expr) => (
        match $it.next() {
            Some(Ok($pattern)) => { $result },
            e => panic!("Ill-formed protocol file: {:?}", e)
        }
    )
);

macro_rules! extract_end_tag(
    ($it: expr => $tag: expr) => (
        extract_from!($it => XmlEvent::EndElement { name } => {
            assert!(name.local_name == $tag, "Ill-formed protocol file");
        });
    )
);

pub fn parse_stream<S: Read>(stream: S) -> Protocol {
    let reader = EventReader::new_with_config(stream, ParserConfig::new().trim_whitespace(true));
    let mut iter = reader.into_iter();
    iter.next(); // StartDocument
    let mut protocol = parse_protocol(iter);

    // yay, hardcoding things
    if protocol.name == "wayland" {
        // wl_callback has actually a destructor *event*, but the wayland specification
        // format does not handle this.
        // Luckily, wayland-scanner does, so we inject it
        for interface in &mut protocol.interfaces {
            if interface.name == "wl_callback" {
                let done_event = &mut interface.events[0];
                assert!(done_event.name == "done");
                done_event.typ = Some(Type::Destructor);
            }
        }
    }

    protocol
}

fn parse_protocol<'a, S: Read + 'a>(mut iter: Events<S>) -> Protocol {
    let mut protocol =
        extract_from!(
        iter => XmlEvent::StartElement { name, attributes, .. } => {
            assert!(name.local_name == "protocol", "Missing protocol toplevel tag");
            assert!(attributes[0].name.local_name == "name", "Protocol must have a name");
            Protocol::new(attributes[0].value.clone())
        }
    );

    loop {
        match iter.next() {
            Some(Ok(XmlEvent::StartElement { name, attributes, .. })) => {
                match &name.local_name[..] {
                    "copyright" => {
                        // parse the copyright
                        let copyright = extract_from!(iter => XmlEvent::Characters(copyright) => copyright);
                        extract_end_tag!(iter => "copyright");
                        protocol.copyright = Some(copyright);
                    }
                    "interface" => {
                        protocol.interfaces.push(
                            parse_interface(&mut iter, attributes),
                        );
                    }
                    "description" => {
                        protocol.description = Some(parse_description(&mut iter, attributes));
                    }
                    _ => {
                        panic!(
                            "Ill-formed protocol file: unexpected token `{}` in protocol {}",
                            name.local_name,
                            protocol.name
                        )
                    }
                }
            }
            Some(Ok(XmlEvent::EndElement { name })) => {
                assert!(
                    name.local_name == "protocol",
                    "Unexpected closing token `{}`",
                    name.local_name
                );
                break;
            }
            e => panic!("Ill-formed protocol file: {:?}", e),
        }
    }

    protocol
}

fn parse_interface<'a, S: Read + 'a>(iter: &mut Events<S>, attrs: Vec<OwnedAttribute>) -> Interface {
    let mut interface = Interface::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "name" => interface.name = attr.value,
            "version" => interface.version = attr.value.parse().unwrap(),
            _ => {}
        }
    }

    loop {
        match iter.next() {
            Some(Ok(XmlEvent::StartElement { name, attributes, .. })) => {
                match &name.local_name[..] {
                    "description" => interface.description = Some(parse_description(iter, attributes)),
                    "request" => interface.requests.push(parse_request(iter, attributes)),
                    "event" => interface.events.push(parse_event(iter, attributes)),
                    "enum" => interface.enums.push(parse_enum(iter, attributes)),
                    _ => panic!("Unexpected tocken: `{}`", name.local_name),
                }
            }
            Some(Ok(XmlEvent::EndElement { ref name })) if name.local_name == "interface" => break,
            _ => {}
        }
    }

    interface
}

fn parse_description<'a, S: Read + 'a>(iter: &mut Events<S>, attrs: Vec<OwnedAttribute>) -> (String, String) {
    let mut summary = String::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "summary" => summary = attr.value.split_whitespace().collect::<Vec<_>>().join(" "),
            _ => {}
        }
    }

    let description = match iter.next() {
        Some(Ok(XmlEvent::Characters(txt))) => {
            extract_end_tag!(iter => "description");
            txt
        }
        Some(Ok(XmlEvent::EndElement { ref name })) if name.local_name == "description" => String::new(),
        e => panic!("Ill-formed protocol file: {:?}", e),
    };

    (summary, description)
}

fn parse_request<'a, S: Read + 'a>(iter: &mut Events<S>, attrs: Vec<OwnedAttribute>) -> Message {
    let mut request = Message::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "name" => request.name = attr.value,
            "type" => request.typ = Some(parse_type(&attr.value)),
            "since" => request.since = attr.value.parse().unwrap(),
            _ => {}
        }
    }

    loop {
        match iter.next() {
            Some(Ok(XmlEvent::StartElement { name, attributes, .. })) => {
                match &name.local_name[..] {
                    "description" => request.description = Some(parse_description(iter, attributes)),
                    "arg" => request.args.push(parse_arg(iter, attributes)),
                    _ => panic!("Unexpected tocken: `{}`", name.local_name),
                }
            }
            Some(Ok(XmlEvent::EndElement { ref name })) if name.local_name == "request" => break,
            _ => {}
        }
    }

    request
}

fn parse_enum<'a, S: Read + 'a>(iter: &mut Events<S>, attrs: Vec<OwnedAttribute>) -> Enum {
    let mut enu = Enum::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "name" => enu.name = attr.value,
            "since" => enu.since = attr.value.parse().unwrap(),
            "bitfield" => {
                if &attr.value[..] == "true" {
                    enu.bitfield = true
                }
            }
            _ => {}
        }
    }

    loop {
        match iter.next() {
            Some(Ok(XmlEvent::StartElement { name, attributes, .. })) => {
                match &name.local_name[..] {
                    "description" => enu.description = Some(parse_description(iter, attributes)),
                    "entry" => enu.entries.push(parse_entry(iter, attributes)),
                    _ => panic!("Unexpected tocken: `{}`", name.local_name),
                }
            }
            Some(Ok(XmlEvent::EndElement { ref name })) if name.local_name == "enum" => break,
            _ => {}
        }
    }

    enu
}

fn parse_event<'a, S: Read + 'a>(iter: &mut Events<S>, attrs: Vec<OwnedAttribute>) -> Message {
    let mut event = Message::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "name" => event.name = attr.value,
            "since" => event.since = attr.value.parse().unwrap(),
            _ => {}
        }
    }

    loop {
        match iter.next() {
            Some(Ok(XmlEvent::StartElement { name, attributes, .. })) => {
                match &name.local_name[..] {
                    "description" => event.description = Some(parse_description(iter, attributes)),
                    "arg" => event.args.push(parse_arg(iter, attributes)),
                    _ => panic!("Unexpected tocken: `{}`", name.local_name),
                }
            }
            Some(Ok(XmlEvent::EndElement { ref name })) if name.local_name == "event" => break,
            _ => {}
        }
    }

    event
}

fn parse_arg<'a, S: Read + 'a>(iter: &mut Events<S>, attrs: Vec<OwnedAttribute>) -> Arg {
    let mut arg = Arg::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "name" => arg.name = attr.value,
            "type" => arg.typ = parse_type(&attr.value),
            "summary" => arg.summary = Some(attr.value),
            "interface" => arg.interface = Some(attr.value),
            "allow-null" => {
                if attr.value == "true" {
                    arg.allow_null = true
                }
            }
            "enum" => arg.enum_ = Some(attr.value),
            _ => {}
        }
    }

    loop {
        match iter.next() {
            Some(Ok(XmlEvent::StartElement { name, attributes, .. })) => {
                match &name.local_name[..] {
                    "description" => arg.description = Some(parse_description(iter, attributes)),
                    _ => panic!("Unexpected tocken: `{}`", name.local_name),
                }
            }
            Some(Ok(XmlEvent::EndElement { ref name })) if name.local_name == "arg" => break,
            _ => {}
        }
    }

    arg
}

fn parse_type(txt: &str) -> Type {
    match txt {
        "int" => Type::Int,
        "uint" => Type::Uint,
        "fixed" => Type::Fixed,
        "string" => Type::String,
        "object" => Type::Object,
        "new_id" => Type::NewId,
        "array" => Type::Array,
        "fd" => Type::Fd,
        "destructor" => Type::Destructor,
        e => panic!("Unexpected type: {}", e),
    }
}

fn parse_entry<'a, S: Read + 'a>(iter: &mut Events<S>, attrs: Vec<OwnedAttribute>) -> Entry {
    let mut entry = Entry::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "name" => entry.name = attr.value,
            "value" => entry.value = attr.value,
            "since" => entry.since = attr.value.parse().unwrap(),
            "summary" => entry.summary = Some(attr.value),
            _ => {}
        }
    }

    loop {
        match iter.next() {
            Some(Ok(XmlEvent::StartElement { name, attributes, .. })) => {
                match &name.local_name[..] {
                    "description" => entry.description = Some(parse_description(iter, attributes)),
                    _ => panic!("Unexpected tocken: `{}`", name.local_name),
                }
            }
            Some(Ok(XmlEvent::EndElement { ref name })) if name.local_name == "entry" => break,
            _ => {}
        }
    }

    entry
}
