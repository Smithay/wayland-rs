
#[derive(Debug)]
pub struct Protocol {
    pub name: String,
    pub copyright: Option<String>,
    pub interfaces: Vec<Interface>
}

impl Protocol {
    pub fn new(name: String) -> Protocol {
        Protocol {
            name: name,
            copyright: None,
            interfaces: Vec::new()
        }
    }
}

#[derive(Debug)]
pub struct Interface {
    pub name: String,
    pub version: u32,
    pub description: Option<String>,
    pub requests: Vec<Request>,
    pub events: Vec<Event>,
    pub enums: Vec<Enum>
}

impl Interface {
    pub fn new() -> Interface {
        Interface {
            name: String::new(),
            version: 0,
            description: None,
            requests: Vec::new(),
            events: Vec::new(),
            enums: Vec::new()
        }
    }
}

#[derive(Debug)]
pub struct Request {
    pub name: String,
    pub typ: Type,
    pub since: u16,
    pub description: Option<String>,
    pub args: Vec<Arg>
}

impl Request {
    pub fn new() -> Request {
        Request {
            name: String::new(),
            typ: Type::Void,
            since: 1,
            description: None,
            args: Vec::new()
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub name: String,
    pub since: u16,
    pub description: Option<String>,
    pub args: Vec<Arg>
}

impl Event {
    pub fn new() -> Event {
        Event {
            name: String::new(),
            since: 1,
            description: None,
            args: Vec::new()
        }
    }
}

#[derive(Debug)]
pub struct Arg {
    pub name: String,
    pub typ: Type,
    pub interface: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub allow_null: bool
}

impl Arg {
    pub fn new() -> Arg {
        Arg {
            name: String::new(),
            typ: Type::Void,
            interface: None,
            summary: None,
            description: None,
            allow_null: false
        }
    }
}

#[derive(Debug)]
pub struct Enum {
    pub name: String,
    pub since: u16,
    pub description: Option<String>,
    pub entries: Vec<Entry>
}

impl Enum {
    pub fn new() -> Enum {
        Enum {
            name: String::new(),
            since: 1,
            description: None,
            entries: Vec::new()
        }
    }
}

#[derive(Debug)]
pub struct Entry {
    pub name: String,
    pub value: String,
    pub since: u16,
    pub description: Option<String>,
    pub summary: Option<String>
}

impl Entry {
    pub fn new() -> Entry {
        Entry {
            name: String::new(),
            value: "0".to_owned(),
            since: 1,
            description: None,
            summary: None,
        }
    }
}

#[derive(Debug)]
pub enum Type {
    Void,
    Int,
    Uint,
    Fixed,
    String,
    Object,
    NewId,
    Array,
    Fd,
    Destructor
}