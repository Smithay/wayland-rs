use crate::protocol::*;
use std::io::Read;
use xml::attribute::OwnedAttribute;
use xml::reader::ParserConfig;
use xml::reader::XmlEvent;
use xml::EventReader;

macro_rules! extract_from(
    ($it: expr => $pattern: pat => $result: expr) => (
        match $it.next() {
            Ok($pattern) => { $result },
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
    let mut reader =
        EventReader::new_with_config(stream, ParserConfig::new().trim_whitespace(true));
    reader.next().expect("Could not read from event reader");
    parse_protocol(reader)
}

fn parse_protocol<R: Read>(mut reader: EventReader<R>) -> Protocol {
    let mut protocol = extract_from!(
        reader => XmlEvent::StartElement { name, attributes, .. } => {
            assert!(name.local_name == "protocol", "Missing protocol toplevel tag");
            assert!(attributes[0].name.local_name == "name", "Protocol must have a name");
            Protocol::new(attributes[0].value.clone())
        }
    );

    loop {
        match reader.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                match &name.local_name[..] {
                    "copyright" => {
                        // parse the copyright
                        let copyright = match reader.next() {
                            Ok(XmlEvent::Characters(copyright))
                            | Ok(XmlEvent::CData(copyright)) => copyright,
                            e => panic!("Ill-formed protocol file: {:?}", e),
                        };

                        extract_end_tag!(reader => "copyright");
                        protocol.copyright = Some(copyright);
                    }
                    "interface" => {
                        protocol.interfaces.push(parse_interface(&mut reader, attributes));
                    }
                    "description" => {
                        protocol.description = Some(parse_description(&mut reader, attributes));
                    }
                    _ => panic!(
                        "Ill-formed protocol file: unexpected token `{}` in protocol {}",
                        name.local_name, protocol.name
                    ),
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
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

fn parse_interface<R: Read>(reader: &mut EventReader<R>, attrs: Vec<OwnedAttribute>) -> Interface {
    let mut interface = Interface::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "name" => interface.name = attr.value,
            "version" => interface.version = attr.value.parse().unwrap(),
            _ => {}
        }
    }

    loop {
        match reader.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => match &name.local_name[..] {
                "description" => {
                    interface.description = Some(parse_description(reader, attributes))
                }
                "request" => interface.requests.push(parse_request(reader, attributes)),
                "event" => interface.events.push(parse_event(reader, attributes)),
                "enum" => interface.enums.push(parse_enum(reader, attributes)),
                _ => panic!("Unexpected tocken: `{}`", name.local_name),
            },
            Ok(XmlEvent::EndElement { ref name }) if name.local_name == "interface" => break,
            _ => {}
        }
    }

    interface
}

fn parse_description<R: Read>(
    reader: &mut EventReader<R>,
    attrs: Vec<OwnedAttribute>,
) -> (String, String) {
    let mut summary = String::new();
    for attr in attrs {
        if &attr.name.local_name[..] == "summary" {
            summary = attr.value.split_whitespace().collect::<Vec<_>>().join(" ");
        }
    }

    let description = match reader.next() {
        Ok(XmlEvent::Characters(txt)) => {
            extract_end_tag!(reader => "description");
            txt
        }
        Ok(XmlEvent::EndElement { ref name }) if name.local_name == "description" => String::new(),
        e => panic!("Ill-formed protocol file: {:?}", e),
    };

    (summary, description)
}

fn parse_request<R: Read>(reader: &mut EventReader<R>, attrs: Vec<OwnedAttribute>) -> Message {
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
        match reader.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => match &name.local_name[..] {
                "description" => request.description = Some(parse_description(reader, attributes)),
                "arg" => request.args.push(parse_arg(reader, attributes)),
                _ => panic!("Unexpected tocken: `{}`", name.local_name),
            },
            Ok(XmlEvent::EndElement { ref name }) if name.local_name == "request" => break,
            _ => {}
        }
    }

    request
}

fn parse_enum<R: Read>(reader: &mut EventReader<R>, attrs: Vec<OwnedAttribute>) -> Enum {
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
        match reader.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => match &name.local_name[..] {
                "description" => enu.description = Some(parse_description(reader, attributes)),
                "entry" => enu.entries.push(parse_entry(reader, attributes)),
                _ => panic!("Unexpected tocken: `{}`", name.local_name),
            },
            Ok(XmlEvent::EndElement { ref name }) if name.local_name == "enum" => break,
            _ => {}
        }
    }

    enu
}

fn parse_event<R: Read>(reader: &mut EventReader<R>, attrs: Vec<OwnedAttribute>) -> Message {
    let mut event = Message::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "name" => event.name = attr.value,
            "type" => event.typ = Some(parse_type(&attr.value)),
            "since" => event.since = attr.value.parse().unwrap(),
            _ => {}
        }
    }

    loop {
        match reader.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => match &name.local_name[..] {
                "description" => event.description = Some(parse_description(reader, attributes)),
                "arg" => event.args.push(parse_arg(reader, attributes)),
                _ => panic!("Unexpected tocken: `{}`", name.local_name),
            },
            Ok(XmlEvent::EndElement { ref name }) if name.local_name == "event" => break,
            _ => {}
        }
    }

    event
}

fn parse_arg<R: Read>(reader: &mut EventReader<R>, attrs: Vec<OwnedAttribute>) -> Arg {
    let mut arg = Arg::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "name" => arg.name = attr.value,
            "type" => arg.typ = parse_type(&attr.value),
            "summary" => {
                arg.summary = Some(attr.value.split_whitespace().collect::<Vec<_>>().join(" "))
            }
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
        match reader.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => match &name.local_name[..] {
                "description" => arg.description = Some(parse_description(reader, attributes)),
                _ => panic!("Unexpected tocken: `{}`", name.local_name),
            },
            Ok(XmlEvent::EndElement { ref name }) if name.local_name == "arg" => break,
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

fn parse_entry<R: Read>(reader: &mut EventReader<R>, attrs: Vec<OwnedAttribute>) -> Entry {
    let mut entry = Entry::new();
    for attr in attrs {
        match &attr.name.local_name[..] {
            "name" => entry.name = attr.value,
            "value" => {
                entry.value = if attr.value.starts_with("0x") {
                    u32::from_str_radix(&attr.value[2..], 16).unwrap()
                } else {
                    attr.value.parse().unwrap()
                };
            }
            "since" => entry.since = attr.value.parse().unwrap(),
            "summary" => {
                entry.summary = Some(attr.value.split_whitespace().collect::<Vec<_>>().join(" "))
            }
            _ => {}
        }
    }

    loop {
        match reader.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => match &name.local_name[..] {
                "description" => entry.description = Some(parse_description(reader, attributes)),
                _ => panic!("Unexpected tocken: `{}`", name.local_name),
            },
            Ok(XmlEvent::EndElement { ref name }) if name.local_name == "entry" => break,
            _ => {}
        }
    }

    entry
}
