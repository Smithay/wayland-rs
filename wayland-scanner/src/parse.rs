use super::protocol::*;
use std::{
    io::{BufRead, BufReader, Read},
    str::FromStr,
};

use quick_xml::{
    Reader,
    events::{Event, attributes::Attributes},
};

pub fn parse<S: Read>(stream: S) -> Protocol {
    let mut reader = Reader::from_reader(BufReader::new(stream));
    let reader_config = reader.config_mut();
    reader_config.trim_text(true);
    reader_config.expand_empty_elements = true;
    parse_protocol(reader)
}

fn parse_bool(txt: &str) -> bool {
    txt == "true"
}

fn parse_or_panic<T: FromStr>(txt: &str) -> T {
    match txt.parse().ok() {
        Some(version) => version,
        None => panic!("Invalid value '{}' for parsing type '{}'", txt, std::any::type_name::<T>()),
    }
}

fn init_protocol<R: BufRead>(reader: &mut Reader<R>) -> Protocol {
    // Check two firsts lines for protocol tag
    for _ in 0..3 {
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Decl(_) | Event::DocType(_) | Event::Comment(_)) => {
                continue;
            }
            Ok(Event::Start(bytes)) => {
                assert!(bytes.name().into_inner() == "protocol", "Missing protocol toplevel tag");
                if let Some(attr) = bytes
                    .attributes()
                    .filter_map(|res| res.ok())
                    .find(|attr| attr.key.into_inner() == "name")
                {
                    return Protocol::new(attr.value.into_owned());
                } else {
                    panic!("Protocol must have a name");
                }
            }
            _ => panic!("Ill-formed protocol file"),
        }
    }
    panic!("Ill-formed protocol file");
}

fn parse_protocol<R: BufRead>(mut reader: Reader<R>) -> Protocol {
    let mut protocol = init_protocol(&mut reader);

    loop {
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Start(bytes)) => {
                match bytes.name().into_inner() {
                    "copyright" => {
                        // parse the copyright
                        let mut copyright = String::new();
                        loop {
                            match reader.read_event_into(&mut Vec::new()) {
                                Ok(Event::Text(text)) => {
                                    copyright.push_str(&text.xml10_content());
                                }
                                Ok(Event::CData(cdata)) => {
                                    copyright.push_str(&cdata.into_inner());
                                }
                                Ok(Event::GeneralRef(byte_ref)) => {
                                    if let Ok(Some(c)) = byte_ref.resolve_char_ref() {
                                        copyright.push(c);
                                    } else {
                                        let content = byte_ref.xml10_content();
                                        if let Some(s) =
                                            quick_xml::escape::resolve_xml_entity(&content)
                                        {
                                            copyright.push_str(s);
                                        }
                                    }
                                }
                                Ok(Event::End(bytes)) => {
                                    assert!(
                                        bytes.name().into_inner() == "copyright",
                                        "Ill-formed protocol file"
                                    );
                                    break;
                                }
                                e => {
                                    panic!("Ill-formed protocol file: {e:?}");
                                }
                            }
                        }

                        protocol.copyright = Some(copyright)
                    }
                    "interface" => {
                        protocol.interfaces.push(parse_interface(&mut reader, bytes.attributes()));
                    }
                    "description" => {
                        protocol.description =
                            Some(parse_description(&mut reader, bytes.attributes()));
                    }
                    name => panic!(
                        "Ill-formed protocol file: unexpected token `{}` in protocol {}",
                        name, protocol.name
                    ),
                }
            }
            Ok(Event::End(bytes)) => {
                let name = bytes.name().into_inner();
                assert!(name == "protocol", "Unexpected closing token `{}`", name);
                break;
            }
            // ignore comments
            Ok(Event::Comment(_)) => {}
            e => panic!("Ill-formed protocol file: unexpected token {e:?}"),
        }
    }

    protocol
}

fn parse_interface<R: BufRead>(reader: &mut Reader<R>, attrs: Attributes) -> Interface {
    let mut interface = Interface::new();
    for attr in attrs.filter_map(|res| res.ok()) {
        match attr.key.into_inner() {
            "name" => interface.name = attr.value.into_owned(),
            "version" => interface.version = parse_or_panic(&attr.value),
            _ => {}
        }
    }

    loop {
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Start(bytes)) => match bytes.name().into_inner() {
                "description" => {
                    interface.description = Some(parse_description(reader, bytes.attributes()))
                }
                "request" => interface.requests.push(parse_request(reader, bytes.attributes())),
                "event" => interface.events.push(parse_event(reader, bytes.attributes())),
                "enum" => interface.enums.push(parse_enum(reader, bytes.attributes())),
                name => panic!("Unexpected token: `{}`", name),
            },
            Ok(Event::End(bytes)) if bytes.name().into_inner() == "interface" => break,
            _ => {}
        }
    }

    interface
}

fn parse_description<R: BufRead>(reader: &mut Reader<R>, attrs: Attributes) -> (String, String) {
    let mut summary = String::new();
    for attr in attrs.filter_map(|res| res.ok()) {
        if attr.key.into_inner() == "summary" {
            summary = attr.value.split_whitespace().collect::<Vec<_>>().join(" ");
        }
    }

    let mut description = String::new();
    // Some protocols have comments inside their descriptions, so we need to parse them in a loop and
    // concatenate the parts into a single block of text
    loop {
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Text(bytes)) => {
                if !description.is_empty() {
                    description.push_str("\n\n");
                }
                description.push_str(&bytes.xml10_content())
            }
            Ok(Event::End(bytes)) if bytes.name().into_inner() == "description" => break,
            Ok(Event::Comment(_)) => {}
            Ok(Event::GeneralRef(byte_ref)) => {
                if let Ok(Some(c)) = byte_ref.resolve_char_ref() {
                    description.push(c);
                } else {
                    let content = byte_ref.xml10_content();
                    if let Some(s) = quick_xml::escape::resolve_xml_entity(&content) {
                        description.push_str(s);
                    }
                }
            }
            Ok(Event::CData(cdata)) => {
                description.push_str(&cdata.into_inner());
            }
            e => panic!("Ill-formed protocol file: {e:?}"),
        }
    }

    (summary, description)
}

fn parse_request<R: BufRead>(reader: &mut Reader<R>, attrs: Attributes) -> Message {
    let mut request = Message::new();
    for attr in attrs.filter_map(|res| res.ok()) {
        match attr.key.into_inner() {
            "name" => request.name = attr.value.into_owned(),
            "type" => request.typ = Some(parse_type(&attr.value)),
            "since" => request.since = parse_or_panic(&attr.value),
            _ => {}
        }
    }

    loop {
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Start(bytes)) => match bytes.name().into_inner() {
                "description" => {
                    request.description = Some(parse_description(reader, bytes.attributes()))
                }
                "arg" => request.args.push(parse_arg(reader, bytes.attributes())),
                name => panic!("Unexpected token: `{}`", name),
            },
            Ok(Event::End(bytes)) if bytes.name().into_inner() == "request" => break,
            _ => {}
        }
    }

    request
}

fn parse_enum<R: BufRead>(reader: &mut Reader<R>, attrs: Attributes) -> Enum {
    let mut enu = Enum::new();
    for attr in attrs.filter_map(|res| res.ok()) {
        match attr.key.into_inner() {
            "name" => enu.name = attr.value.into_owned(),
            "since" => enu.since = parse_or_panic(&attr.value),
            "bitfield" => enu.bitfield = parse_bool(&attr.value),
            _ => {}
        }
    }

    loop {
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Start(bytes)) => match bytes.name().into_inner() {
                "description" => {
                    enu.description = Some(parse_description(reader, bytes.attributes()))
                }
                "entry" => enu.entries.push(parse_entry(reader, bytes.attributes())),
                name => panic!("Unexpected token: `{}`", name),
            },
            Ok(Event::End(bytes)) if bytes.name().into_inner() == "enum" => break,
            _ => {}
        }
    }

    enu
}

fn parse_event<R: BufRead>(reader: &mut Reader<R>, attrs: Attributes) -> Message {
    let mut event = Message::new();
    for attr in attrs.filter_map(|res| res.ok()) {
        match attr.key.into_inner() {
            "name" => event.name = attr.value.into_owned(),
            "type" => event.typ = Some(parse_type(&attr.value)),
            "since" => event.since = parse_or_panic(&attr.value),
            _ => {}
        }
    }

    loop {
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Start(bytes)) => match bytes.name().into_inner() {
                "description" => {
                    event.description = Some(parse_description(reader, bytes.attributes()))
                }
                "arg" => event.args.push(parse_arg(reader, bytes.attributes())),
                name => panic!("Unexpected token: `{}`", name),
            },
            Ok(Event::End(bytes)) if bytes.name().into_inner() == "event" => break,
            _ => {}
        }
    }

    event
}

fn parse_enum_relname_or_panic(value: String) -> EnumRef {
    let mut iter = value.rsplit('.');
    let name = iter.next().unwrap().to_string();
    let interface = iter.next().map(|s| s.to_string());
    if iter.next().is_some() {
        panic!("Invalid relname: '{value}'")
    }
    EnumRef { interface, name }
}

fn parse_arg<R: BufRead>(reader: &mut Reader<R>, attrs: Attributes) -> Arg {
    let mut arg = Arg::new();
    for attr in attrs.filter_map(|res| res.ok()) {
        match attr.key.into_inner() {
            "name" => arg.name = attr.value.into_owned(),
            "type" => arg.typ = parse_type(&attr.value),
            "summary" => {
                arg.summary = Some(attr.value.split_whitespace().collect::<Vec<_>>().join(" "))
            }
            "interface" => arg.interface = Some(parse_or_panic(&attr.value)),
            "allow-null" => arg.allow_null = parse_bool(&attr.value),
            "enum" => arg.enum_ = Some(parse_enum_relname_or_panic(attr.value.into_owned())),
            _ => {}
        }
    }

    loop {
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Start(bytes)) => match bytes.name().into_inner() {
                "description" => {
                    arg.description = Some(parse_description(reader, bytes.attributes()))
                }
                name => panic!("Unexpected token: `{}`", name),
            },
            Ok(Event::End(bytes)) if bytes.name().into_inner() == "arg" => break,
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

fn parse_entry<R: BufRead>(reader: &mut Reader<R>, attrs: Attributes) -> Entry {
    let mut entry = Entry::new();
    for attr in attrs.filter_map(|res| res.ok()) {
        match attr.key.into_inner() {
            "name" => entry.name = attr.value.into_owned(),
            "value" => {
                entry.value = if attr.value.starts_with("0x") {
                    if let Ok(val) = u32::from_str_radix(&attr.value[2..], 16) {
                        val
                    } else {
                        panic!("Invalid number: {}", attr.value)
                    }
                } else {
                    parse_or_panic(&attr.value)
                };
            }
            "since" => entry.since = parse_or_panic(&attr.value),
            "summary" => {
                entry.summary = Some(attr.value.split_whitespace().collect::<Vec<_>>().join(" "))
            }
            _ => {}
        }
    }

    loop {
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Start(bytes)) => match bytes.name().into_inner() {
                "description" => {
                    entry.description = Some(parse_description(reader, bytes.attributes()))
                }
                name => panic!("Unexpected token: `{}`", name),
            },
            Ok(Event::End(bytes)) if bytes.name().into_inner() == "entry" => break,
            _ => {}
        }
    }

    entry
}

#[cfg(test)]
mod tests {
    #[test]
    fn xml_parse() {
        let protocol_file =
            std::fs::File::open("./tests/scanner_assets/test-protocol.xml").unwrap();
        let _ = crate::parse::parse(protocol_file);
    }

    #[test]
    fn headerless_xml_parse() {
        let protocol_file =
            std::fs::File::open("./tests/scanner_assets/test-headerless-protocol.xml").unwrap();
        let _ = crate::parse::parse(protocol_file);
    }
}
