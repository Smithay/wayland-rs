use std::os::raw::{c_char, c_void};
const NULLPTR: *const c_void = 0 as *const c_void;
static mut types_null: [*const sys::common::wl_interface; 8] = [
    NULLPTR as *const sys::common::wl_interface,
    NULLPTR as *const sys::common::wl_interface,
    NULLPTR as *const sys::common::wl_interface,
    NULLPTR as *const sys::common::wl_interface,
    NULLPTR as *const sys::common::wl_interface,
    NULLPTR as *const sys::common::wl_interface,
    NULLPTR as *const sys::common::wl_interface,
    NULLPTR as *const sys::common::wl_interface,
];
#[doc = "Interface for fooing\n\nThis is the dedicated interface for doing foos over any\nkind of other foos."]
pub mod wl_foo {
    use super::sys::client::*;
    use super::sys::common::{wl_argument, wl_array, wl_interface, wl_message};
    use super::{
        smallvec, types_null, AnonymousObject, Argument, ArgumentType, Interface, Main, Message, MessageDesc,
        MessageGroup, Object, ObjectMetadata, Proxy, NULLPTR,
    };
    use std::os::raw::c_char;
    #[doc = "Possible cake kinds\n\nList of the possible kind of cake supported by the protocol."]
    #[repr(u32)]
    #[derive(Copy, Clone, Debug, PartialEq)]
    #[non_exhaustive]
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
    #[non_exhaustive]
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
        CreateBar {},
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
                destructor: false,
            },
            super::MessageDesc {
                name: "create_bar",
                since: 1,
                signature: &[super::ArgumentType::NewId],
                destructor: false,
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
        fn since(&self) -> u32 {
            match *self {
                Request::FooIt { .. } => 1,
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
                    args: smallvec![
                        Argument::Int(number),
                        Argument::Uint(unumber),
                        Argument::Str(Box::new(unsafe {
                            ::std::ffi::CString::from_vec_unchecked(text.into())
                        })),
                        Argument::Fixed((float * 256.) as i32),
                        Argument::Fd(file),
                    ],
                },
                Request::CreateBar {} => Message {
                    sender_id: sender_id,
                    opcode: 1,
                    args: smallvec![Argument::NewId(0),],
                },
            }
        }
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Request, ()> {
            panic!("Request::from_raw_c can not be used Client-side.")
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            match self {
                Request::FooIt {
                    number,
                    unumber,
                    text,
                    float,
                    file,
                } => {
                    let mut _args_array: [wl_argument; 5] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].i = number;
                    _args_array[1].u = unumber;
                    let _arg_2 = ::std::ffi::CString::new(text).unwrap();
                    _args_array[2].s = _arg_2.as_ptr();
                    _args_array[3].f = (float * 256.) as i32;
                    _args_array[4].h = file;
                    f(0, &mut _args_array)
                }
                Request::CreateBar {} => {
                    let mut _args_array: [wl_argument; 1] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].o = ::std::ptr::null_mut() as *mut _;
                    f(1, &mut _args_array)
                }
            }
        }
    }
    #[non_exhaustive]
    pub enum Event {
        #[doc = "a cake is possible\n\nThe server advertises that a kind of cake is available\n\nOnly available since version 2 of the interface"]
        Cake { kind: CakeKind, amount: u32 },
    }
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[super::MessageDesc {
            name: "cake",
            since: 2,
            signature: &[super::ArgumentType::Uint, super::ArgumentType::Uint],
            destructor: false,
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
        fn since(&self) -> u32 {
            match *self {
                Event::Cake { .. } => 2,
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
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Event, ()> {
            match opcode {
                0 => {
                    let _args = ::std::slice::from_raw_parts(args, 2);
                    Ok(Event::Cake {
                        kind: CakeKind::from_raw(_args[0].u).ok_or(())?,
                        amount: _args[1].u,
                    })
                }
                _ => return Err(()),
            }
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            panic!("Event::as_raw_c_in can not be used Client-side.")
        }
    }
    #[derive(Clone, Eq, PartialEq)]
    pub struct WlFoo(Proxy<WlFoo>);
    impl AsRef<Proxy<WlFoo>> for WlFoo {
        #[inline]
        fn as_ref(&self) -> &Proxy<Self> {
            &self.0
        }
    }
    impl From<Proxy<WlFoo>> for WlFoo {
        #[inline]
        fn from(value: Proxy<Self>) -> Self {
            WlFoo(value)
        }
    }
    impl From<WlFoo> for Proxy<WlFoo> {
        #[inline]
        fn from(value: WlFoo) -> Self {
            value.0
        }
    }
    impl Interface for WlFoo {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_foo";
        const VERSION: u32 = 3;
        fn c_interface() -> *const wl_interface {
            unsafe { &wl_foo_interface }
        }
    }
    impl WlFoo {
        #[doc = "do some foo\n\nThis will do some foo with its args."]
        pub fn foo_it(
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
            self.0.send::<AnonymousObject>(msg, None);
        }
        #[doc = "create a bar\n\nCreate a bar which will do its bar job."]
        pub fn create_bar(&self) -> Main<super::wl_bar::WlBar> {
            let msg = Request::CreateBar {};
            self.0.send(msg, None).unwrap()
        }
    }
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_FOO_IT_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_CREATE_BAR_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_CAKE_SINCE: u32 = 2u32;
    static mut wl_foo_requests_create_bar_types: [*const wl_interface; 1] =
        [unsafe { &super::wl_bar::wl_bar_interface as *const wl_interface }];
    #[doc = r" C-representation of the messages of this interface, for interop"]
    pub static mut wl_foo_requests: [wl_message; 2] = [
        wl_message {
            name: b"foo_it\0" as *const u8 as *const c_char,
            signature: b"iusfh\0" as *const u8 as *const c_char,
            types: unsafe { &types_null as *const _ },
        },
        wl_message {
            name: b"create_bar\0" as *const u8 as *const c_char,
            signature: b"n\0" as *const u8 as *const c_char,
            types: unsafe { &wl_foo_requests_create_bar_types as *const _ },
        },
    ];
    #[doc = r" C-representation of the messages of this interface, for interop"]
    pub static mut wl_foo_events: [wl_message; 1] = [wl_message {
        name: b"cake\0" as *const u8 as *const c_char,
        signature: b"2uu\0" as *const u8 as *const c_char,
        types: unsafe { &types_null as *const _ },
    }];
    #[doc = r" C representation of this interface, for interop"]
    pub static mut wl_foo_interface: wl_interface = wl_interface {
        name: b"wl_foo\0" as *const u8 as *const c_char,
        version: 3,
        request_count: 2,
        requests: unsafe { &wl_foo_requests as *const _ },
        event_count: 1,
        events: unsafe { &wl_foo_events as *const _ },
    };
}
#[doc = "Interface for bars\n\nThis interface allows you to bar your foos."]
pub mod wl_bar {
    use super::sys::client::*;
    use super::sys::common::{wl_argument, wl_array, wl_interface, wl_message};
    use super::{
        smallvec, types_null, AnonymousObject, Argument, ArgumentType, Interface, Main, Message, MessageDesc,
        MessageGroup, Object, ObjectMetadata, Proxy, NULLPTR,
    };
    use std::os::raw::c_char;
    #[non_exhaustive]
    pub enum Request {
        #[doc = "ask for a bar delivery\n\nProceed to a bar delivery of given foo.\n\nOnly available since version 2 of the interface"]
        BarDelivery {
            kind: super::wl_foo::DeliveryKind,
            target: super::wl_foo::WlFoo,
            metadata: Vec<u8>,
            metametadata: Option<Vec<u8>>,
        },
        #[doc = "release this bar\n\nNotify the compositor that you have finished using this bar.\n\nThis is a destructor, once sent this object cannot be used any longer."]
        Release,
        #[doc = "ask for erronous bindings from wayland-scanner\n\nThis request tests argument names which can break wayland-scanner.\n\nOnly available since version 2 of the interface"]
        _Self {
            _self: u32,
            _mut: u32,
            object: u32,
            ___object: u32,
            handler: u32,
            ___handler: u32,
            request: u32,
            event: u32,
        },
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
                destructor: false,
            },
            super::MessageDesc {
                name: "release",
                since: 1,
                signature: &[],
                destructor: true,
            },
            super::MessageDesc {
                name: "self",
                since: 2,
                signature: &[
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                    super::ArgumentType::Uint,
                ],
                destructor: false,
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
                Request::_Self { .. } => 2,
            }
        }
        fn since(&self) -> u32 {
            match *self {
                Request::BarDelivery { .. } => 2,
                Request::Release => 1,
                Request::_Self { .. } => 2,
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
                    args: smallvec![
                        Argument::Uint(kind.to_raw()),
                        Argument::Object(target.as_ref().id()),
                        Argument::Array(Box::new(metadata)),
                        Argument::Array(Box::new(metametadata.unwrap_or_else(Vec::new))),
                    ],
                },
                Request::Release => Message {
                    sender_id: sender_id,
                    opcode: 1,
                    args: smallvec![],
                },
                Request::_Self {
                    _self,
                    _mut,
                    object,
                    ___object,
                    handler,
                    ___handler,
                    request,
                    event,
                } => Message {
                    sender_id: sender_id,
                    opcode: 2,
                    args: smallvec![
                        Argument::Uint(_self),
                        Argument::Uint(_mut),
                        Argument::Uint(object),
                        Argument::Uint(___object),
                        Argument::Uint(handler),
                        Argument::Uint(___handler),
                        Argument::Uint(request),
                        Argument::Uint(event),
                    ],
                },
            }
        }
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Request, ()> {
            panic!("Request::from_raw_c can not be used Client-side.")
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            match self {
                Request::BarDelivery {
                    kind,
                    target,
                    metadata,
                    metametadata,
                } => {
                    let mut _args_array: [wl_argument; 4] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = kind.to_raw();
                    _args_array[1].o = target.as_ref().c_ptr() as *mut _;
                    let _arg_2 = wl_array {
                        size: metadata.len(),
                        alloc: metadata.capacity(),
                        data: metadata.as_ptr() as *mut _,
                    };
                    _args_array[2].a = &_arg_2;
                    let _arg_3 = metametadata.as_ref().map(|vec| wl_array {
                        size: vec.len(),
                        alloc: vec.capacity(),
                        data: vec.as_ptr() as *mut _,
                    });
                    _args_array[3].a = _arg_3
                        .as_ref()
                        .map(|a| a as *const wl_array)
                        .unwrap_or(::std::ptr::null());
                    f(0, &mut _args_array)
                }
                Request::Release => {
                    let mut _args_array: [wl_argument; 0] = unsafe { ::std::mem::zeroed() };
                    f(1, &mut _args_array)
                }
                Request::_Self {
                    _self,
                    _mut,
                    object,
                    ___object,
                    handler,
                    ___handler,
                    request,
                    event,
                } => {
                    let mut _args_array: [wl_argument; 8] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = _self;
                    _args_array[1].u = _mut;
                    _args_array[2].u = object;
                    _args_array[3].u = ___object;
                    _args_array[4].u = handler;
                    _args_array[5].u = ___handler;
                    _args_array[6].u = request;
                    _args_array[7].u = event;
                    f(2, &mut _args_array)
                }
            }
        }
    }
    #[non_exhaustive]
    pub enum Event {
        #[doc = "ask for erronous bindings from wayland-scanner\n\nThis event tests argument names which can break wayland-scanner.\n\nOnly available since version 2 of the interface"]
        _Self {
            _self: u32,
            _mut: u32,
            object: u32,
            ___object: u32,
            handler: u32,
            ___handler: u32,
            request: u32,
            event: u32,
        },
    }
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[super::MessageDesc {
            name: "self",
            since: 2,
            signature: &[
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
                super::ArgumentType::Uint,
            ],
            destructor: false,
        }];
        type Map = super::ProxyMap;
        fn is_destructor(&self) -> bool {
            match *self {
                _ => false,
            }
        }
        fn opcode(&self) -> u16 {
            match *self {
                Event::_Self { .. } => 0,
            }
        }
        fn since(&self) -> u32 {
            match *self {
                Event::_Self { .. } => 2,
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
                    Ok(Event::_Self {
                        _self: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        _mut: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        object: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        ___object: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        handler: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        ___handler: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        request: {
                            if let Some(Argument::Uint(val)) = args.next() {
                                val
                            } else {
                                return Err(());
                            }
                        },
                        event: {
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
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Event, ()> {
            match opcode {
                0 => {
                    let _args = ::std::slice::from_raw_parts(args, 8);
                    Ok(Event::_Self {
                        _self: _args[0].u,
                        _mut: _args[1].u,
                        object: _args[2].u,
                        ___object: _args[3].u,
                        handler: _args[4].u,
                        ___handler: _args[5].u,
                        request: _args[6].u,
                        event: _args[7].u,
                    })
                }
                _ => return Err(()),
            }
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            panic!("Event::as_raw_c_in can not be used Client-side.")
        }
    }
    #[derive(Clone, Eq, PartialEq)]
    pub struct WlBar(Proxy<WlBar>);
    impl AsRef<Proxy<WlBar>> for WlBar {
        #[inline]
        fn as_ref(&self) -> &Proxy<Self> {
            &self.0
        }
    }
    impl From<Proxy<WlBar>> for WlBar {
        #[inline]
        fn from(value: Proxy<Self>) -> Self {
            WlBar(value)
        }
    }
    impl From<WlBar> for Proxy<WlBar> {
        #[inline]
        fn from(value: WlBar) -> Self {
            value.0
        }
    }
    impl Interface for WlBar {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_bar";
        const VERSION: u32 = 1;
        fn c_interface() -> *const wl_interface {
            unsafe { &wl_bar_interface }
        }
    }
    impl WlBar {
        #[doc = "ask for a bar delivery\n\nProceed to a bar delivery of given foo.\n\nOnly available since version 2 of the interface."]
        pub fn bar_delivery(
            &self,
            kind: super::wl_foo::DeliveryKind,
            target: &super::wl_foo::WlFoo,
            metadata: Vec<u8>,
            metametadata: Option<Vec<u8>>,
        ) -> () {
            let msg = Request::BarDelivery {
                kind: kind,
                target: target.clone(),
                metadata: metadata,
                metametadata: metametadata,
            };
            self.0.send::<AnonymousObject>(msg, None);
        }
        #[doc = "release this bar\n\nNotify the compositor that you have finished using this bar.\n\nThis is a destructor, you cannot send requests to this object any longer once this method is called."]
        pub fn release(&self) -> () {
            let msg = Request::Release;
            self.0.send::<AnonymousObject>(msg, None);
        }
        #[doc = "ask for erronous bindings from wayland-scanner\n\nThis request tests argument names which can break wayland-scanner.\n\nOnly available since version 2 of the interface."]
        pub fn _self(
            &self,
            _self: u32,
            _mut: u32,
            object: u32,
            ___object: u32,
            handler: u32,
            ___handler: u32,
            request: u32,
            event: u32,
        ) -> () {
            let msg = Request::_Self {
                _self: _self,
                _mut: _mut,
                object: object,
                ___object: ___object,
                handler: handler,
                ___handler: ___handler,
                request: request,
                event: event,
            };
            self.0.send::<AnonymousObject>(msg, None);
        }
    }
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_BAR_DELIVERY_SINCE: u32 = 2u32;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_RELEASE_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_SELF_SINCE: u32 = 2u32;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_SELF_SINCE: u32 = 2u32;
    static mut wl_bar_requests_bar_delivery_types: [*const wl_interface; 4] = [
        NULLPTR as *const wl_interface,
        unsafe { &super::wl_foo::wl_foo_interface as *const wl_interface },
        NULLPTR as *const wl_interface,
        NULLPTR as *const wl_interface,
    ];
    #[doc = r" C-representation of the messages of this interface, for interop"]
    pub static mut wl_bar_requests: [wl_message; 3] = [
        wl_message {
            name: b"bar_delivery\0" as *const u8 as *const c_char,
            signature: b"2uoa?a\0" as *const u8 as *const c_char,
            types: unsafe { &wl_bar_requests_bar_delivery_types as *const _ },
        },
        wl_message {
            name: b"release\0" as *const u8 as *const c_char,
            signature: b"\0" as *const u8 as *const c_char,
            types: unsafe { &types_null as *const _ },
        },
        wl_message {
            name: b"self\0" as *const u8 as *const c_char,
            signature: b"2uuuuuuuu\0" as *const u8 as *const c_char,
            types: unsafe { &types_null as *const _ },
        },
    ];
    #[doc = r" C-representation of the messages of this interface, for interop"]
    pub static mut wl_bar_events: [wl_message; 1] = [wl_message {
        name: b"self\0" as *const u8 as *const c_char,
        signature: b"2uuuuuuuu\0" as *const u8 as *const c_char,
        types: unsafe { &types_null as *const _ },
    }];
    #[doc = r" C representation of this interface, for interop"]
    pub static mut wl_bar_interface: wl_interface = wl_interface {
        name: b"wl_bar\0" as *const u8 as *const c_char,
        version: 1,
        request_count: 3,
        requests: unsafe { &wl_bar_requests as *const _ },
        event_count: 1,
        events: unsafe { &wl_bar_events as *const _ },
    };
}
#[doc = "core global object\n\nThis global is special and should only generate code client-side, not server-side."]
pub mod wl_display {
    use super::sys::client::*;
    use super::sys::common::{wl_argument, wl_array, wl_interface, wl_message};
    use super::{
        smallvec, types_null, AnonymousObject, Argument, ArgumentType, Interface, Main, Message, MessageDesc,
        MessageGroup, Object, ObjectMetadata, Proxy, NULLPTR,
    };
    use std::os::raw::c_char;
    #[non_exhaustive]
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
        fn since(&self) -> u32 {
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
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Request, ()> {
            panic!("Request::from_raw_c can not be used Client-side.")
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            match self {}
        }
    }
    #[non_exhaustive]
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
        fn since(&self) -> u32 {
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
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Event, ()> {
            match opcode {
                _ => return Err(()),
            }
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            panic!("Event::as_raw_c_in can not be used Client-side.")
        }
    }
    #[derive(Clone, Eq, PartialEq)]
    pub struct WlDisplay(Proxy<WlDisplay>);
    impl AsRef<Proxy<WlDisplay>> for WlDisplay {
        #[inline]
        fn as_ref(&self) -> &Proxy<Self> {
            &self.0
        }
    }
    impl From<Proxy<WlDisplay>> for WlDisplay {
        #[inline]
        fn from(value: Proxy<Self>) -> Self {
            WlDisplay(value)
        }
    }
    impl From<WlDisplay> for Proxy<WlDisplay> {
        #[inline]
        fn from(value: WlDisplay) -> Self {
            value.0
        }
    }
    impl Interface for WlDisplay {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_display";
        const VERSION: u32 = 1;
        fn c_interface() -> *const wl_interface {
            unsafe { &wl_display_interface }
        }
    }
    impl WlDisplay {}
    #[doc = r" C representation of this interface, for interop"]
    pub static mut wl_display_interface: wl_interface = wl_interface {
        name: b"wl_display\0" as *const u8 as *const c_char,
        version: 1,
        request_count: 0,
        requests: NULLPTR as *const wl_message,
        event_count: 0,
        events: NULLPTR as *const wl_message,
    };
}
#[doc = "global registry object\n\nThis global is special and should only generate code client-side, not server-side."]
pub mod wl_registry {
    use super::sys::client::*;
    use super::sys::common::{wl_argument, wl_array, wl_interface, wl_message};
    use super::{
        smallvec, types_null, AnonymousObject, Argument, ArgumentType, Interface, Main, Message, MessageDesc,
        MessageGroup, Object, ObjectMetadata, Proxy, NULLPTR,
    };
    use std::os::raw::c_char;
    #[non_exhaustive]
    pub enum Request {
        #[doc = "bind an object to the display\n\nThis request is a special code-path, as its new-id argument as no target type."]
        Bind { name: u32, id: (String, u32) },
    }
    impl super::MessageGroup for Request {
        const MESSAGES: &'static [super::MessageDesc] = &[super::MessageDesc {
            name: "bind",
            since: 1,
            signature: &[super::ArgumentType::Uint, super::ArgumentType::NewId],
            destructor: false,
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
        fn since(&self) -> u32 {
            match *self {
                Request::Bind { .. } => 1,
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
                    args: smallvec![
                        Argument::Uint(name),
                        Argument::Str(Box::new(unsafe {
                            ::std::ffi::CString::from_vec_unchecked(id.0.into())
                        })),
                        Argument::Uint(id.1),
                        Argument::NewId(0),
                    ],
                },
            }
        }
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Request, ()> {
            panic!("Request::from_raw_c can not be used Client-side.")
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            match self {
                Request::Bind { name, id } => {
                    let mut _args_array: [wl_argument; 4] = unsafe { ::std::mem::zeroed() };
                    _args_array[0].u = name;
                    let _arg_1_s = ::std::ffi::CString::new(id.0).unwrap();
                    _args_array[1].s = _arg_1_s.as_ptr();
                    _args_array[2].u = id.1;
                    _args_array[3].o = ::std::ptr::null_mut();
                    f(0, &mut _args_array)
                }
            }
        }
    }
    #[non_exhaustive]
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
        fn since(&self) -> u32 {
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
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Event, ()> {
            match opcode {
                _ => return Err(()),
            }
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            panic!("Event::as_raw_c_in can not be used Client-side.")
        }
    }
    #[derive(Clone, Eq, PartialEq)]
    pub struct WlRegistry(Proxy<WlRegistry>);
    impl AsRef<Proxy<WlRegistry>> for WlRegistry {
        #[inline]
        fn as_ref(&self) -> &Proxy<Self> {
            &self.0
        }
    }
    impl From<Proxy<WlRegistry>> for WlRegistry {
        #[inline]
        fn from(value: Proxy<Self>) -> Self {
            WlRegistry(value)
        }
    }
    impl From<WlRegistry> for Proxy<WlRegistry> {
        #[inline]
        fn from(value: WlRegistry) -> Self {
            value.0
        }
    }
    impl Interface for WlRegistry {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_registry";
        const VERSION: u32 = 1;
        fn c_interface() -> *const wl_interface {
            unsafe { &wl_registry_interface }
        }
    }
    impl WlRegistry {
        #[doc = "bind an object to the display\n\nThis request is a special code-path, as its new-id argument as no target type."]
        pub fn bind<T: Interface + From<Proxy<T>> + AsRef<Proxy<T>>>(
            &self,
            version: u32,
            name: u32,
        ) -> Main<T> {
            let msg = Request::Bind {
                name: name,
                id: (T::NAME.into(), version),
            };
            self.0.send(msg, Some(version)).unwrap()
        }
    }
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_BIND_SINCE: u32 = 1u32;
    #[doc = r" C-representation of the messages of this interface, for interop"]
    pub static mut wl_registry_requests: [wl_message; 1] = [wl_message {
        name: b"bind\0" as *const u8 as *const c_char,
        signature: b"usun\0" as *const u8 as *const c_char,
        types: unsafe { &types_null as *const _ },
    }];
    #[doc = r" C representation of this interface, for interop"]
    pub static mut wl_registry_interface: wl_interface = wl_interface {
        name: b"wl_registry\0" as *const u8 as *const c_char,
        version: 1,
        request_count: 1,
        requests: unsafe { &wl_registry_requests as *const _ },
        event_count: 0,
        events: NULLPTR as *const wl_message,
    };
}
#[doc = "callback object\n\nThis object has a special behavior regarding its destructor."]
pub mod wl_callback {
    use super::sys::client::*;
    use super::sys::common::{wl_argument, wl_array, wl_interface, wl_message};
    use super::{
        smallvec, types_null, AnonymousObject, Argument, ArgumentType, Interface, Main, Message, MessageDesc,
        MessageGroup, Object, ObjectMetadata, Proxy, NULLPTR,
    };
    use std::os::raw::c_char;
    #[non_exhaustive]
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
        fn since(&self) -> u32 {
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
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Request, ()> {
            panic!("Request::from_raw_c can not be used Client-side.")
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            match self {}
        }
    }
    #[non_exhaustive]
    pub enum Event {
        #[doc = "done event\n\nThis event is actually a destructor, but the protocol XML has no way of specifying it.\nAs such, the scanner should consider wl_callback.done as a special case.\n\nThis is a destructor, once received this object cannot be used any longer."]
        Done { callback_data: u32 },
    }
    impl super::MessageGroup for Event {
        const MESSAGES: &'static [super::MessageDesc] = &[super::MessageDesc {
            name: "done",
            since: 1,
            signature: &[super::ArgumentType::Uint],
            destructor: true,
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
        fn since(&self) -> u32 {
            match *self {
                Event::Done { .. } => 1,
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
        unsafe fn from_raw_c(
            obj: *mut ::std::os::raw::c_void,
            opcode: u32,
            args: *const wl_argument,
        ) -> Result<Event, ()> {
            match opcode {
                0 => {
                    let _args = ::std::slice::from_raw_parts(args, 1);
                    Ok(Event::Done {
                        callback_data: _args[0].u,
                    })
                }
                _ => return Err(()),
            }
        }
        fn as_raw_c_in<F, T>(self, f: F) -> T
        where
            F: FnOnce(u32, &mut [wl_argument]) -> T,
        {
            panic!("Event::as_raw_c_in can not be used Client-side.")
        }
    }
    #[derive(Clone, Eq, PartialEq)]
    pub struct WlCallback(Proxy<WlCallback>);
    impl AsRef<Proxy<WlCallback>> for WlCallback {
        #[inline]
        fn as_ref(&self) -> &Proxy<Self> {
            &self.0
        }
    }
    impl From<Proxy<WlCallback>> for WlCallback {
        #[inline]
        fn from(value: Proxy<Self>) -> Self {
            WlCallback(value)
        }
    }
    impl From<WlCallback> for Proxy<WlCallback> {
        #[inline]
        fn from(value: WlCallback) -> Self {
            value.0
        }
    }
    impl Interface for WlCallback {
        type Request = Request;
        type Event = Event;
        const NAME: &'static str = "wl_callback";
        const VERSION: u32 = 1;
        fn c_interface() -> *const wl_interface {
            unsafe { &wl_callback_interface }
        }
    }
    impl WlCallback {}
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_DONE_SINCE: u32 = 1u32;
    #[doc = r" C-representation of the messages of this interface, for interop"]
    pub static mut wl_callback_events: [wl_message; 1] = [wl_message {
        name: b"done\0" as *const u8 as *const c_char,
        signature: b"u\0" as *const u8 as *const c_char,
        types: unsafe { &types_null as *const _ },
    }];
    #[doc = r" C representation of this interface, for interop"]
    pub static mut wl_callback_interface: wl_interface = wl_interface {
        name: b"wl_callback\0" as *const u8 as *const c_char,
        version: 1,
        request_count: 0,
        requests: NULLPTR as *const wl_message,
        event_count: 1,
        events: unsafe { &wl_callback_events as *const _ },
    };
}
