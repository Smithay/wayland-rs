#[doc = "Interface for fooing\n\nThis is the dedicated interface for doing foos over any\nkind of other foos."]
pub mod wl_foo {
    use super::{
        AnonymousObject, Argument, ArgumentType, Interface, Message, MessageDesc, MessageGroup, NewProxy,
        Object, ObjectMetadata, Proxy,
    };
    #[doc = "Possible cake kinds\n\nList of the possible kind of cake supported by the protocol."]
    #[repr(u32)]
    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum CakeKind {
        #[doc = "mild cake without much flavor"]
        Basic = 0,
        #[doc = "spicy cake to burn your tongue"]
        Spicy = 1,
        #[doc = "fruity cake to get vitamins"]
        Fruity = 2,
    }
    impl CakeKind {
        pub fn from_raw(n: u32) -> Option<CakeKind> {
            match n {
                0 => Some(CakeKind::Basic),
                1 => Some(CakeKind::Spicy),
                2 => Some(CakeKind::Fruity),
                _ => Option::None,
            }
        }
        pub fn to_raw(&self) -> u32 {
            *self as u32
        }
    }
    bitflags! { # [ doc = "possible delivery modes" ] pub struct DeliveryKind : u32 { # [ doc = "pick your cake up yourself" ] const PickUp = 1 ; # [ doc = "flying drone delivery" ] const Drone = 2 ; # [ doc = "because we fear nothing" ] const Catapult = 4 ; } }
    impl DeliveryKind {
        pub fn from_raw(n: u32) -> Option<DeliveryKind> {
            Some(DeliveryKind::from_bits_truncate(n))
        }
        pub fn to_raw(&self) -> u32 {
            self.bits()
        }
    }
    pub enum Request {
        #[doc = "do some foo\n\nThis will do some foo with its args."]
        FooIt {
            number: i32,
            unumber: u32,
            text: String,
            float: f64,
            file: ::std::os::unix::io::RawFd,
        },
        #[doc = "create a bar\n\nCreate a bar which will do its bar job."]
        CreateBar { id: Proxy<super::wl_bar::WlBar> },
    }
    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[
            super::MessageDesc {
                name: "foo_it",
                since: 1,
                signature: &[
                    super::ArgumentType::Int,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Str,
                    super::ArgumentType::Fixed,
                    super::ArgumentType::Fd,
                ],
            },
            super::MessageDesc {
                name: "create_bar",
                since: 1,
                signature: &[super::ArgumentType::NewId],
            },
        ];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Request::FooIt { .. } => 0,
                Request::CreateBar { .. } => 1,
            }
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                1 => Some(Object::from_interface::<super::wl_bar::WlBar>(
                    version,
                    meta.child(),
                )),
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Request::from_raw can not be used Client-side.")
        }
        fn into_raw(self, sender_id: u32) -> Message {
            match self {
                Request::FooIt {
                    number,
                    unumber,
                    text,
                    float,
                    file,
                } => Message {
                    sender_id: sender_id,
                    opcode: 0,
                    args: vec![
                        Argument::Int(number),
                        Argument::Uint(unumber),
                        Argument::Str(unsafe { ::std::ffi::CString::from_vec_unchecked(text.into()) }),
                        Argument::Fixed((float * 256.) as i32),
                        Argument::Fd(file),
                    ],
                },
                Request::CreateBar { id } => Message {
                    sender_id: sender_id,
                    opcode: 1,
                    args: vec![Argument::NewId(id.id())],
                },
            }
        }
    }
    pub enum Event {
        #[doc = "a cake is possible\n\nThe server advertises that a kind of cake is available\n\nOnly available since version 2 of the interface"]
        Cake { kind: CakeKind, amount: u32 },
    }
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[super::MessageDesc {
            name: "cake",
            since: 2,
            signature: &[super::ArgumentType::Uint, super::ArgumentType::Uint],
        }];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Event::Cake { .. } => 0,
            }
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                0 => {
                    let mut args = msg.args.into_iter();
                    Ok(Event::Cake {
                        kind: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                CakeKind::from_raw(val).ok_or(())?
                            } else {
                                return Err(());
                            }
                        },
                        amount: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                    })
                }
                _ => Err(()),
            }
        }
        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Event::into_raw can not be used Client-side.")
        }
    }
    pub struct WlFoo;
    impl Interface for WlFoo {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_foo";
        const VERSION: u32 = 3;
    }
    pub trait RequestsTrait {
        #[doc = "do some foo\n\nThis will do some foo with its args."]
        fn foo_it(
            &self,
            number: i32,
            unumber: u32,
            text: String,
            float: f64,
            file: ::std::os::unix::io::RawFd,
        ) -> ();
        #[doc = "create a bar\n\nCreate a bar which will do its bar job."]
        fn create_bar<F>(&self, implementor: F) -> Result<Proxy<super::wl_bar::WlBar>, ()>
        where
            F: FnOnce(NewProxy<super::wl_bar::WlBar>) -> Proxy<super::wl_bar::WlBar>;
    }
    impl RequestsTrait for Proxy<WlFoo> {
        fn foo_it(
            &self,
            number: i32,
            unumber: u32,
            text: String,
            float: f64,
            file: ::std::os::unix::io::RawFd,
        ) -> () {
            let msg = Request::FooIt {
                number: number,
                unumber: unumber,
                text: text,
                float: float,
                file: file,
            };
            self.send(msg);
        }
        fn create_bar<F>(&self, implementor: F) -> Result<Proxy<super::wl_bar::WlBar>, ()>
        where
            F: FnOnce(NewProxy<super::wl_bar::WlBar>) -> Proxy<super::wl_bar::WlBar>,
        {
            let msg = Request::CreateBar {
                id: self.child_placeholder(),
            };
            self.send_constructor(msg, implementor, None)
        }
    }
}
#[doc = "Interface for bars\n\nThis interface allows you to bar your foos."]
pub mod wl_bar {
    use super::{
        AnonymousObject, Argument, ArgumentType, Interface, Message, MessageDesc, MessageGroup, NewProxy,
        Object, ObjectMetadata, Proxy,
    };
    pub enum Request {
        #[doc = "ask for a bar delivery\n\nProceed to a bar delivery of given foo.\n\nOnly available since version 2 of the interface"]
        BarDelivery {
            kind: super::wl_foo::DeliveryKind,
            target: Proxy<super::wl_foo::WlFoo>,
            metadata: Vec<u8>,
            metametadata: Option<Vec<u8>>,
        },
        #[doc = "release this bar\n\nNotify the compositor that you have finished using this bar.\n\nThis is a destructor, once sent this object cannot be used any longer."]
        Release,
    }
    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[
            super::MessageDesc {
                name: "bar_delivery",
                since: 2,
                signature: &[
                    super::ArgumentType::Uint,
                    super::ArgumentType::Object,
                    super::ArgumentType::Array,
                    super::ArgumentType::Array,
                ],
            },
            super::MessageDesc {
                name: "release",
                since: 1,
                signature: &[],
            },
        ];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {
                Request::Release => true,
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Request::BarDelivery { .. } => 0,
                Request::Release => 1,
            }
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Request::from_raw can not be used Client-side.")
        }
        fn into_raw(self, sender_id: u32) -> Message {
            match self {
                Request::BarDelivery {
                    kind,
                    target,
                    metadata,
                    metametadata,
                } => Message {
                    sender_id: sender_id,
                    opcode: 0,
                    args: vec![
                        Argument::Uint(kind.to_raw()),
                        Argument::Object(target.id()),
                        Argument::Array(metadata),
                        Argument::Array(metametadata.unwrap_or_else(Vec::new)),
                    ],
                },
                Request::Release => Message {
                    sender_id: sender_id,
                    opcode: 1,
                    args: vec![],
                },
            }
        }
    }
    pub enum Event {}
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {}
        }
        fn opcode(&self) -> u16 {
            match *self {}
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                _ => Err(()),
            }
        }
        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Event::into_raw can not be used Client-side.")
        }
    }
    pub struct WlBar;
    impl Interface for WlBar {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_bar";
        const VERSION: u32 = 1;
    }
    pub trait RequestsTrait {
        #[doc = "ask for a bar delivery\n\nProceed to a bar delivery of given foo.\n\nOnly available since version 2 of the interface."]
        fn bar_delivery(
            &self,
            kind: super::wl_foo::DeliveryKind,
            target: &Proxy<super::wl_foo::WlFoo>,
            metadata: Vec<u8>,
            metametadata: Option<Vec<u8>>,
        ) -> ();
        #[doc = "release this bar\n\nNotify the compositor that you have finished using this bar.\n\nThis is a destructor, you cannot send requests to this object any longer once this method is called."]
        fn release(&self) -> ();
    }
    impl RequestsTrait for Proxy<WlBar> {
        fn bar_delivery(
            &self,
            kind: super::wl_foo::DeliveryKind,
            target: &Proxy<super::wl_foo::WlFoo>,
            metadata: Vec<u8>,
            metametadata: Option<Vec<u8>>,
        ) -> () {
            let msg = Request::BarDelivery {
                kind: kind,
                target: target.clone(),
                metadata: metadata,
                metametadata: metametadata,
            };
            self.send(msg);
        }
        fn release(&self) -> () {
            let msg = Request::Release;
            self.send(msg);
        }
    }
}
#[doc = "core global object\n\nThis global is special and should only generate code client-side, not server-side."]
pub mod wl_display {
    use super::{
        AnonymousObject, Argument, ArgumentType, Interface, Message, MessageDesc, MessageGroup, NewProxy,
        Object, ObjectMetadata, Proxy,
    };
    pub enum Request {}
    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {}
        }
        fn opcode(&self) -> u16 {
            match *self {}
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Request::from_raw can not be used Client-side.")
        }
        fn into_raw(self, sender_id: u32) -> Message {
            match self {}
        }
    }
    pub enum Event {}
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {}
        }
        fn opcode(&self) -> u16 {
            match *self {}
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                _ => Err(()),
            }
        }
        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Event::into_raw can not be used Client-side.")
        }
    }
    pub struct WlDisplay;
    impl Interface for WlDisplay {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_display";
        const VERSION: u32 = 1;
    }
    pub trait RequestsTrait {}
    impl RequestsTrait for Proxy<WlDisplay> {}
}
#[doc = "global registry object\n\nThis global is special and should only generate code client-side, not server-side."]
pub mod wl_registry {
    use super::{
        AnonymousObject, Argument, ArgumentType, Interface, Message, MessageDesc, MessageGroup, NewProxy,
        Object, ObjectMetadata, Proxy,
    };
    pub enum Request {
        #[doc = "bind an object to the display\n\nThis request is a special code-path, as its new-id argument as no target type."]
        Bind {
            name: u32,
            id: (String, u32, Proxy<AnonymousObject>),
        },
    }
    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[super::MessageDesc {
            name: "bind",
            since: 1,
            signature: &[super::ArgumentType::Uint, super::ArgumentType::NewId],
        }];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Request::Bind { .. } => 0,
            }
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Request::from_raw can not be used Client-side.")
        }
        fn into_raw(self, sender_id: u32) -> Message {
            match self {
                Request::Bind { name, id } => Message {
                    sender_id: sender_id,
                    opcode: 0,
                    args: vec![
                        Argument::Uint(name),
                        Argument::Str(unsafe { ::std::ffi::CString::from_vec_unchecked(id.0.into()) }),
                        Argument::Uint(id.1),
                        Argument::NewId(id.2.id()),
                    ],
                },
            }
        }
    }
    pub enum Event {}
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {}
        }
        fn opcode(&self) -> u16 {
            match *self {}
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                _ => Err(()),
            }
        }
        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Event::into_raw can not be used Client-side.")
        }
    }
    pub struct WlRegistry;
    impl Interface for WlRegistry {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_registry";
        const VERSION: u32 = 1;
    }
    pub trait RequestsTrait {
        #[doc = "bind an object to the display\n\nThis request is a special code-path, as its new-id argument as no target type."]
        fn bind<T: Interface, F>(&self, version: u32, name: u32, implementor: F) -> Result<Proxy<T>, ()>
        where
            F: FnOnce(NewProxy<T>) -> Proxy<T>;
    }
    impl RequestsTrait for Proxy<WlRegistry> {
        fn bind<T: Interface, F>(&self, version: u32, name: u32, implementor: F) -> Result<Proxy<T>, ()>
        where
            F: FnOnce(NewProxy<T>) -> Proxy<T>,
        {
            let msg = Request::Bind {
                name: name,
                id: (T::NAME.into(), version, self.child_placeholder()),
            };
            self.send_constructor(msg, implementor, Some(version))
        }
    }
}
#[doc = "callback object\n\nThis object has a special behavior regarding its destructor."]
pub mod wl_callback {
    use super::{
        AnonymousObject, Argument, ArgumentType, Interface, Message, MessageDesc, MessageGroup, NewProxy,
        Object, ObjectMetadata, Proxy,
    };
    pub enum Request {}
    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {}
        }
        fn opcode(&self) -> u16 {
            match *self {}
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Request::from_raw can not be used Client-side.")
        }
        fn into_raw(self, sender_id: u32) -> Message {
            match self {}
        }
    }
    pub enum Event {
        #[doc = "done event\n\nThis event is actually a destructor, but the protocol XML has no way of specifying it.\nAs such, the scanner should consider wl_callback.done as a special case.\n\nThis is a destructor, once received this object cannot be used any longer."]
        Done { callback_data: u32 },
    }
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[super::MessageDesc {
            name: "done",
            since: 1,
            signature: &[super::ArgumentType::Uint],
        }];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {
                Event::Done { .. } => true,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Event::Done { .. } => 0,
            }
        }
        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None,
            }
        }
        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                0 => {
                    let mut args = msg.args.into_iter();
                    Ok(Event::Done {
                        callback_data: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                    })
                }
                _ => Err(()),
            }
        }
        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Event::into_raw can not be used Client-side.")
        }
    }
    pub struct WlCallback;
    impl Interface for WlCallback {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_callback";
        const VERSION: u32 = 1;
    }
    pub trait RequestsTrait {}
    impl RequestsTrait for Proxy<WlCallback> {}
}
