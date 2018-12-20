
//
// This file was auto-generated, do not edit directly.
//

/*
This is an example copyright.
    It contains several lines.
    AS WELL AS ALL CAPS TEXT.
*/

pub mod wl_foo {
    //! Interface for fooing
    //!
    //! This is the dedicated interface for doing foos over any
    //! kind of other foos.
    use super::{Resource, NewResource, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType, Object, Message, Argument, ObjectMetadata};

    /// Possible cake kinds
    ///
    /// List of the possible kind of cake supported by the protocol.

    #[repr(u32)]
    #[derive(Copy,Clone,Debug,PartialEq)]
    pub enum CakeKind {
        /// mild cake without much flavor
        Basic = 0,
        /// spicy cake to burn your tongue
        Spicy = 1,
        /// fruity cake to get vitamins
        Fruity = 2,
    }
    impl CakeKind {
        pub fn from_raw(n: u32) -> Option<CakeKind> {
            match n {
                0 => Some(CakeKind::Basic),
                1 => Some(CakeKind::Spicy),
                2 => Some(CakeKind::Fruity),

                _ => Option::None
            }
        }
        pub fn to_raw(&self) -> u32 {
            *self as u32
        }
    }

    bitflags! {
        /// possible delivery modes
        ///
        pub struct DeliveryKind: u32 {
            /// pick your cake up yourself
            const PickUp = 1;
            /// flying drone delivery
            const Drone = 2;
            /// because we fear nothing
            const Catapult = 4;
        }
    }
    impl DeliveryKind {
        pub fn from_raw(n: u32) -> Option<DeliveryKind> {
            Some(DeliveryKind::from_bits_truncate(n))

        }
        pub fn to_raw(&self) -> u32 {
            self.bits()
        }
    }

    pub enum Request {
        /// do some foo
        ///
        /// This will do some foo with its args.
        FooIt {number: i32, unumber: u32, text: String, float: f64, file: ::std::os::unix::io::RawFd, },
        /// create a bar
        ///
        /// Create a bar which will do its bar job.
        CreateBar {id: NewResource<super::wl_bar::WlBar>, },
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
                ]
            },
            super::MessageDesc {
                name: "create_bar",
                since: 1,
                signature: &[
                    super::ArgumentType::NewId,
                ]
            },
        ];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
                _ => false
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
                1 => Some(Object::from_interface::<super::wl_bar::WlBar>(version, meta.child())),
                _ => None
            }
        }

        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                0 => {
                    let mut args = msg.args.into_iter();
                    Ok(Request::FooIt {
                        number: {
                            if let Some(Argument::Int(val)) = args.next() {
                                val
                            } else {
                                return Err(())
                            }
                        },
                        unumber: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(())
                            }
                        },
                        text: {
                            if let Some(Argument::Str(val)) = args.next() {
                                let s = String::from_utf8(val.into_bytes()).unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into());
                                s
                            } else {
                                return Err(())
                            }
                        },
                        float: {
                            if let Some(Argument::Fixed(val)) = args.next() {
                                (val as f64) / 256.
                            } else {
                                return Err(())
                            }
                        },
                        file: {
                            if let Some(Argument::Fd(val)) = args.next() {
                                val
                            } else {
                                return Err(())
                            }
                        },
                    })
                },
                1 => {
                    let mut args = msg.args.into_iter();
                    Ok(Request::CreateBar {
                        id: {
                            if let Some(Argument::NewId(val)) = args.next() {
                                map.get_new(val).ok_or(())?
                            } else {
                                return Err(())
                            }
                        },
                    })
                },
                _ => Err(()),
            }
        }

        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Request::into_raw can not be used Server-side.")
        }

    }

    pub enum Event {
        /// a cake is possible
        ///
        /// The server advertises that a kind of cake is available
        ///
        /// Only available since version 2 of the interface
        Cake {kind: CakeKind, amount: u32, },
    }

    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[
            super::MessageDesc {
                name: "cake",
                since: 2,
                signature: &[
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                ]
            },
        ];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
                _ => false
            }
        }

        fn opcode(&self) -> u16 {
            match *self {
                Event::Cake { .. } => 0,
            }
        }

        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None
            }
        }

        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Event::from_raw can not be used Server-side.")
        }

        fn into_raw(self, sender_id: u32) -> Message {
            match self {
                Event::Cake { kind, amount, } => Message {
                    sender_id: sender_id,
                    opcode: 0,
                    args: vec![
                        Argument::Uint(kind.to_raw()),
                        Argument::Uint(amount),
                    ]
                },
            }
        }

    }


    pub struct WlFoo;

    impl Interface for WlFoo {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_foo";
        const VERSION: u32 = 3;

    }
}

pub mod wl_bar {
    //! Interface for bars
    //!
    //! This interface allows you to bar your foos.
    use super::{Resource, NewResource, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType, Object, Message, Argument, ObjectMetadata};

    pub enum Request {
        /// ask for a bar delivery
        ///
        /// Proceed to a bar delivery of given foo.
        ///
        /// Only available since version 2 of the interface
        BarDelivery {kind: super::wl_foo::DeliveryKind, target: Resource<super::wl_foo::WlFoo>, metadata: Vec<u8>, metametadata: Option<Vec<u8>>, },
        /// release this bar
        ///
        /// Notify the compositor that you have finished using this bar.
        ///
        /// This is a destructor, once received this object cannot be used any longer.
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
                ]
            },
            super::MessageDesc {
                name: "release",
                since: 1,
                signature: &[
                ]
            },
        ];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
                Request::Release => true,
                _ => false
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
                _ => None
            }
        }

        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                0 => {
                    let mut args = msg.args.into_iter();
                    Ok(Request::BarDelivery {
                        kind: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                super::wl_foo::DeliveryKind::from_raw(val).ok_or(())?
                            } else {
                                return Err(())
                            }
                        },
                        target: {
                            if let Some(Argument::Object(val)) = args.next() {
                                map.get(val).ok_or(())?
                            } else {
                                return Err(())
                            }
                        },
                        metadata: {
                            if let Some(Argument::Array(val)) = args.next() {
                                val
                            } else {
                                return Err(())
                            }
                        },
                        metametadata: {
                            if let Some(Argument::Array(val)) = args.next() {
                                if val.len() == 0 { None } else { Some(val) }
                            } else {
                                return Err(())
                            }
                        },
                    })
                },
                1 => {
                    Ok(Request::Release)
                },
                _ => Err(()),
            }
        }

        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Request::into_raw can not be used Server-side.")
        }

    }

    pub enum Event {
    }

    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[
        ];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
            }
        }

        fn opcode(&self) -> u16 {
            match *self {
            }
        }

        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None
            }
        }

        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Event::from_raw can not be used Server-side.")
        }

        fn into_raw(self, sender_id: u32) -> Message {
            match self {
            }
        }

    }


    pub struct WlBar;

    impl Interface for WlBar {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_bar";
        const VERSION: u32 = 1;

    }
}

pub mod wl_callback {
    //! callback object
    //!
    //! This object has a special behavior regarding its destructor.
    use super::{Resource, NewResource, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType, Object, Message, Argument, ObjectMetadata};

    pub enum Request {
    }

    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[
        ];
        type Map = super::ResourceMap;
        fn is_destructor(&self) -> bool {
            match *self {
            }
        }

        fn opcode(&self) -> u16 {
            match *self {
            }
        }

        fn child<Meta: ObjectMetadata>(opcode: u16, version: u32, meta: &Meta) -> Option<Object<Meta>> {
            match opcode {
                _ => None
            }
        }

        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            match msg.opcode {
                _ => Err(()),
            }
        }

        fn into_raw(self, sender_id: u32) -> Message {
            panic!("Request::into_raw can not be used Server-side.")
        }

    }

    pub enum Event {
        /// done event
        ///
        /// This event is actually a destructor, but the protocol XML has no wait of specifying it.
        /// As such, the scanner should consider wl_callback.done as a special case.
        ///
        /// This is a destructor, once sent this object cannot be used any longer.
        Done {callback_data: u32, },
    }

    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[
            super::MessageDesc {
                name: "done",
                since: 1,
                signature: &[
                    super::ArgumentType::Uint,
                ]
            },
        ];
        type Map = super::ResourceMap;
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
                _ => None
            }
        }

        fn from_raw(msg: Message, map: &mut Self::Map) -> Result<Self, ()> {
            panic!("Event::from_raw can not be used Server-side.")
        }

        fn into_raw(self, sender_id: u32) -> Message {
            match self {
                Event::Done { callback_data, } => Message {
                    sender_id: sender_id,
                    opcode: 0,
                    args: vec![
                        Argument::Uint(callback_data),
                    ]
                },
            }
        }

    }


    pub struct WlCallback;

    impl Interface for WlCallback {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_callback";
        const VERSION: u32 = 1;

    }
}

