#[doc = "core global object\n\nThe core global object.  This is a special singleton object.  It\nis used for internal Wayland protocol features."]
pub mod wl_display {
    use super::wayland_client::{
        backend::{
            protocol::{same_interface, Argument, Interface, Message, WEnum},
            smallvec, InvalidId, ObjectData, ObjectId,
        },
        ConnectionHandle, Dispatch, DispatchError, Proxy, QueueHandle, QueueProxyData,
    };
    use std::sync::Arc;
    #[doc = "global error values\n\nThese errors are global and can be emitted in response to any\nserver request."]
    #[repr(u32)]
    #[derive(Copy, Clone, Debug, PartialEq)]
    #[non_exhaustive]
    pub enum Error {
        #[doc = "server couldn't find object"]
        InvalidObject = 0,
        #[doc = "method doesn't exist on the specified interface or malformed request"]
        InvalidMethod = 1,
        #[doc = "server is out of memory"]
        NoMemory = 2,
        #[doc = "implementation error in compositor"]
        Implementation = 3,
    }
    impl std::convert::TryFrom<u32> for Error {
        type Error = ();
        fn try_from(val: u32) -> Result<Error, ()> {
            match val {
                0 => Ok(Error::InvalidObject),
                1 => Ok(Error::InvalidMethod),
                2 => Ok(Error::NoMemory),
                3 => Ok(Error::Implementation),
                _ => Err(()),
            }
        }
    }
    impl std::convert::From<Error> for u32 {
        fn from(val: Error) -> u32 {
            val as u32
        }
    }
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_SYNC_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_GET_REGISTRY_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_ERROR_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_DELETE_ID_SINCE: u32 = 1u32;
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Request {
        #[doc = "asynchronous roundtrip\n\nThe sync request asks the server to emit the 'done' event\non the returned wl_callback object.  Since requests are\nhandled in-order and events are delivered in-order, this can\nbe used as a barrier to ensure all previous requests and the\nresulting events have been handled.\n\nThe object returned by this request will be destroyed by the\ncompositor after the callback is fired and as such the client must not\nattempt to use it after that point.\n\nThe callback_data passed in the callback is the event serial."]
        Sync {},
        #[doc = "get global registry object\n\nThis request creates a registry object that allows the client\nto list and bind the global objects available from the\ncompositor.\n\nIt should be noted that the server side resources consumed in\nresponse to a get_registry request can only be released when the\nclient disconnects, not when the client side proxy is destroyed.\nTherefore, clients should invoke get_registry as infrequently as\npossible to avoid wasting memory."]
        GetRegistry {},
    }
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Event {
        #[doc = "fatal error event\n\nThe error event is sent out when a fatal (non-recoverable)\nerror has occurred.  The object_id argument is the object\nwhere the error occurred, most often in response to a request\nto that object.  The code identifies the error and is defined\nby the object interface.  As such, each interface defines its\nown set of error codes.  The message is a brief description\nof the error, for (debugging) convenience."]
        Error { object_id: super::wayland_client::ObjectId, code: u32, message: String },
        #[doc = "acknowledge object ID deletion\n\nThis event is used internally by the object ID management\nlogic. When a client deletes an object that it had created,\nthe server will send this event to acknowledge that it has\nseen the delete request. When the client receives this event,\nit will know that it can safely reuse the object ID."]
        DeleteId { id: u32 },
    }
    #[derive(Debug, Clone)]
    pub struct WlDisplay {
        id: ObjectId,
        version: u32,
        data: Option<Arc<dyn ObjectData>>,
    }
    impl std::cmp::PartialEq for WlDisplay {
        fn eq(&self, other: &WlDisplay) -> bool {
            self.id == other.id
        }
    }
    impl std::cmp::Eq for WlDisplay {}
    impl super::wayland_client::Proxy for WlDisplay {
        type Request = Request;
        type Event = Event;
        #[inline]
        fn interface() -> &'static Interface {
            &super::WL_DISPLAY_INTERFACE
        }
        #[inline]
        fn id(&self) -> ObjectId {
            self.id.clone()
        }
        #[inline]
        fn version(&self) -> u32 {
            self.version
        }
        #[inline]
        fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
            self.data
                .as_ref()
                .and_then(|arc| (&**arc).downcast_ref::<QueueProxyData<Self, U>>())
                .map(|data| &data.udata)
        }
        #[inline]
        fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId> {
            if !same_interface(id.interface(), Self::interface()) && !id.is_null() {
                return Err(InvalidId);
            }
            let version = cx.object_info(id.clone()).map(|info| info.version).unwrap_or(0);
            let data = cx.get_object_data(id.clone()).ok();
            Ok(WlDisplay { id, data, version })
        }
        fn parse_event(
            cx: &mut ConnectionHandle,
            msg: Message<ObjectId>,
        ) -> Result<(Self, Self::Event), DispatchError> {
            let me = Self::from_id(cx, msg.sender_id.clone()).unwrap();
            match msg.opcode {
                0u16 => {
                    if let [Argument::Object(object_id), Argument::Uint(code), Argument::Str(message)] =
                        &msg.args[..]
                    {
                        Ok((
                            me,
                            Event::Error {
                                object_id: object_id.clone(),
                                code: *code,
                                message: String::from_utf8_lossy(message.as_bytes()).into_owned(),
                            },
                        ))
                    } else {
                        Err(DispatchError::BadMessage { msg, interface: Self::interface().name })
                    }
                }
                1u16 => {
                    if let [Argument::Uint(id)] = &msg.args[..] {
                        Ok((me, Event::DeleteId { id: *id }))
                    } else {
                        Err(DispatchError::BadMessage { msg, interface: Self::interface().name })
                    }
                }
                _ => Err(DispatchError::BadMessage { msg, interface: Self::interface().name }),
            }
        }
        fn write_request(
            &self,
            cx: &mut ConnectionHandle,
            msg: Self::Request,
        ) -> Result<Message<ObjectId>, InvalidId> {
            match msg {
                Request::Sync {} => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 0u16,
                    args: smallvec::smallvec![{
                        let my_info = cx.object_info(self.id())?;
                        let placeholder = cx.placeholder_id(Some((
                            super::wl_callback::WlCallback::interface(),
                            my_info.version,
                        )));
                        Argument::NewId(placeholder)
                    }],
                }),
                Request::GetRegistry {} => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 1u16,
                    args: smallvec::smallvec![{
                        let my_info = cx.object_info(self.id())?;
                        let placeholder = cx.placeholder_id(Some((
                            super::wl_registry::WlRegistry::interface(),
                            my_info.version,
                        )));
                        Argument::NewId(placeholder)
                    }],
                }),
            }
        }
    }
    impl WlDisplay {
        #[allow(clippy::too_many_arguments)]
        pub fn sync<D: Dispatch<super::wl_callback::WlCallback> + 'static>(
            &self,
            cx: &mut ConnectionHandle,
            qh: &QueueHandle<D>,
            udata: <D as Dispatch<super::wl_callback::WlCallback>>::UserData,
        ) -> Result<super::wl_callback::WlCallback, InvalidId> {
            let ret = cx.send_request(
                self,
                Request::Sync {},
                Some(qh.make_data::<super::wl_callback::WlCallback>(udata)),
            )?;
            Proxy::from_id(cx, ret)
        }
        #[allow(clippy::too_many_arguments)]
        pub fn get_registry<D: Dispatch<super::wl_registry::WlRegistry> + 'static>(
            &self,
            cx: &mut ConnectionHandle,
            qh: &QueueHandle<D>,
            udata: <D as Dispatch<super::wl_registry::WlRegistry>>::UserData,
        ) -> Result<super::wl_registry::WlRegistry, InvalidId> {
            let ret = cx.send_request(
                self,
                Request::GetRegistry {},
                Some(qh.make_data::<super::wl_registry::WlRegistry>(udata)),
            )?;
            Proxy::from_id(cx, ret)
        }
    }
}
#[doc = "global registry object\n\nThe singleton global registry object.  The server has a number of\nglobal objects that are available to all clients.  These objects\ntypically represent an actual object in the server (for example,\nan input device) or they are singleton objects that provide\nextension functionality.\n\nWhen a client creates a registry object, the registry object\nwill emit a global event for each global currently in the\nregistry.  Globals come and go as a result of device or\nmonitor hotplugs, reconfiguration or other events, and the\nregistry will send out global and global_remove events to\nkeep the client up to date with the changes.  To mark the end\nof the initial burst of events, the client can use the\nwl_display.sync request immediately after calling\nwl_display.get_registry.\n\nA client can bind to a global object by using the bind\nrequest.  This creates a client-side handle that lets the object\nemit events to the client and lets the client invoke requests on\nthe object."]
pub mod wl_registry {
    use super::wayland_client::{
        backend::{
            protocol::{same_interface, Argument, Interface, Message, WEnum},
            smallvec, InvalidId, ObjectData, ObjectId,
        },
        ConnectionHandle, Dispatch, DispatchError, Proxy, QueueHandle, QueueProxyData,
    };
    use std::sync::Arc;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_BIND_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_GLOBAL_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_GLOBAL_REMOVE_SINCE: u32 = 1u32;
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Request {
        #[doc = "bind an object to the display\n\nBinds a new, client-created object to the server using the\nspecified name as the identifier."]
        Bind { name: u32, id: (&'static Interface, u32) },
    }
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Event {
        #[doc = "announce global object\n\nNotify the client of global objects.\n\nThe event notifies the client that a global object with\nthe given name is now available, and it implements the\ngiven version of the given interface."]
        Global { name: u32, interface: String, version: u32 },
        #[doc = "announce removal of global object\n\nNotify the client of removed global objects.\n\nThis event notifies the client that the global identified\nby name is no longer available.  If the client bound to\nthe global using the bind request, the client should now\ndestroy that object.\n\nThe object remains valid and requests to the object will be\nignored until the client destroys it, to avoid races between\nthe global going away and a client sending a request to it."]
        GlobalRemove { name: u32 },
    }
    #[derive(Debug, Clone)]
    pub struct WlRegistry {
        id: ObjectId,
        version: u32,
        data: Option<Arc<dyn ObjectData>>,
    }
    impl std::cmp::PartialEq for WlRegistry {
        fn eq(&self, other: &WlRegistry) -> bool {
            self.id == other.id
        }
    }
    impl std::cmp::Eq for WlRegistry {}
    impl super::wayland_client::Proxy for WlRegistry {
        type Request = Request;
        type Event = Event;
        #[inline]
        fn interface() -> &'static Interface {
            &super::WL_REGISTRY_INTERFACE
        }
        #[inline]
        fn id(&self) -> ObjectId {
            self.id.clone()
        }
        #[inline]
        fn version(&self) -> u32 {
            self.version
        }
        #[inline]
        fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
            self.data
                .as_ref()
                .and_then(|arc| (&**arc).downcast_ref::<QueueProxyData<Self, U>>())
                .map(|data| &data.udata)
        }
        #[inline]
        fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId> {
            if !same_interface(id.interface(), Self::interface()) && !id.is_null() {
                return Err(InvalidId);
            }
            let version = cx.object_info(id.clone()).map(|info| info.version).unwrap_or(0);
            let data = cx.get_object_data(id.clone()).ok();
            Ok(WlRegistry { id, data, version })
        }
        fn parse_event(
            cx: &mut ConnectionHandle,
            msg: Message<ObjectId>,
        ) -> Result<(Self, Self::Event), DispatchError> {
            let me = Self::from_id(cx, msg.sender_id.clone()).unwrap();
            match msg.opcode {
                0u16 => {
                    if let [Argument::Uint(name), Argument::Str(interface), Argument::Uint(version)] =
                        &msg.args[..]
                    {
                        Ok((
                            me,
                            Event::Global {
                                name: *name,
                                interface: String::from_utf8_lossy(interface.as_bytes())
                                    .into_owned(),
                                version: *version,
                            },
                        ))
                    } else {
                        Err(DispatchError::BadMessage { msg, interface: Self::interface().name })
                    }
                }
                1u16 => {
                    if let [Argument::Uint(name)] = &msg.args[..] {
                        Ok((me, Event::GlobalRemove { name: *name }))
                    } else {
                        Err(DispatchError::BadMessage { msg, interface: Self::interface().name })
                    }
                }
                _ => Err(DispatchError::BadMessage { msg, interface: Self::interface().name }),
            }
        }
        fn write_request(
            &self,
            cx: &mut ConnectionHandle,
            msg: Self::Request,
        ) -> Result<Message<ObjectId>, InvalidId> {
            match msg {
                Request::Bind { name, id } => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 0u16,
                    args: smallvec::smallvec![
                        Argument::Uint(name),
                        Argument::Str(Box::new(std::ffi::CString::new(id.0.name).unwrap())),
                        Argument::Uint(id.1),
                        Argument::NewId(cx.placeholder_id(Some((id.0, id.1))))
                    ],
                }),
            }
        }
    }
    impl WlRegistry {
        #[allow(clippy::too_many_arguments)]
        pub fn bind<I: Proxy + 'static, D: Dispatch<I> + 'static>(
            &self,
            cx: &mut ConnectionHandle,
            name: u32,
            version: u32,
            qh: &QueueHandle<D>,
            udata: <D as Dispatch<I>>::UserData,
        ) -> Result<I, InvalidId> {
            let placeholder = cx.placeholder_id(Some((I::interface(), version)));
            let ret = cx.send_request(
                self,
                Request::Bind { name, id: (I::interface(), version) },
                Some(qh.make_data::<I>(udata)),
            )?;
            Proxy::from_id(cx, ret)
        }
    }
}
#[doc = "callback object\n\nClients can handle the 'done' event to get notified when\nthe related request is done."]
pub mod wl_callback {
    use super::wayland_client::{
        backend::{
            protocol::{same_interface, Argument, Interface, Message, WEnum},
            smallvec, InvalidId, ObjectData, ObjectId,
        },
        ConnectionHandle, Dispatch, DispatchError, Proxy, QueueHandle, QueueProxyData,
    };
    use std::sync::Arc;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_DONE_SINCE: u32 = 1u32;
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Request {}
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Event {
        #[doc = "done event\n\nNotify the client when the related request is done.\n\nThis is a destructor, once received this object cannot be used any longer."]
        Done { callback_data: u32 },
    }
    #[derive(Debug, Clone)]
    pub struct WlCallback {
        id: ObjectId,
        version: u32,
        data: Option<Arc<dyn ObjectData>>,
    }
    impl std::cmp::PartialEq for WlCallback {
        fn eq(&self, other: &WlCallback) -> bool {
            self.id == other.id
        }
    }
    impl std::cmp::Eq for WlCallback {}
    impl super::wayland_client::Proxy for WlCallback {
        type Request = Request;
        type Event = Event;
        #[inline]
        fn interface() -> &'static Interface {
            &super::WL_CALLBACK_INTERFACE
        }
        #[inline]
        fn id(&self) -> ObjectId {
            self.id.clone()
        }
        #[inline]
        fn version(&self) -> u32 {
            self.version
        }
        #[inline]
        fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
            self.data
                .as_ref()
                .and_then(|arc| (&**arc).downcast_ref::<QueueProxyData<Self, U>>())
                .map(|data| &data.udata)
        }
        #[inline]
        fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId> {
            if !same_interface(id.interface(), Self::interface()) && !id.is_null() {
                return Err(InvalidId);
            }
            let version = cx.object_info(id.clone()).map(|info| info.version).unwrap_or(0);
            let data = cx.get_object_data(id.clone()).ok();
            Ok(WlCallback { id, data, version })
        }
        fn parse_event(
            cx: &mut ConnectionHandle,
            msg: Message<ObjectId>,
        ) -> Result<(Self, Self::Event), DispatchError> {
            let me = Self::from_id(cx, msg.sender_id.clone()).unwrap();
            match msg.opcode {
                0u16 => {
                    if let [Argument::Uint(callback_data)] = &msg.args[..] {
                        Ok((me, Event::Done { callback_data: *callback_data }))
                    } else {
                        Err(DispatchError::BadMessage { msg, interface: Self::interface().name })
                    }
                }
                _ => Err(DispatchError::BadMessage { msg, interface: Self::interface().name }),
            }
        }
        fn write_request(
            &self,
            cx: &mut ConnectionHandle,
            msg: Self::Request,
        ) -> Result<Message<ObjectId>, InvalidId> {
            match msg {}
        }
    }
    impl WlCallback {}
}
pub mod test_global {
    use super::wayland_client::{
        backend::{
            protocol::{same_interface, Argument, Interface, Message, WEnum},
            smallvec, InvalidId, ObjectData, ObjectId,
        },
        ConnectionHandle, Dispatch, DispatchError, Proxy, QueueHandle, QueueProxyData,
    };
    use std::sync::Arc;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_MANY_ARGS_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_GET_SECONDARY_SINCE: u32 = 2u32;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_GET_TERTIARY_SINCE: u32 = 3u32;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_LINK_SINCE: u32 = 3u32;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_DESTROY_SINCE: u32 = 4u32;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_MANY_ARGS_EVT_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_ACK_SECONDARY_SINCE: u32 = 1u32;
    #[doc = r" The minimal object version supporting this event"]
    pub const EVT_CYCLE_QUAD_SINCE: u32 = 1u32;
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Request {
        #[doc = "a request with every possible non-object arg"]
        ManyArgs {
            unsigned_int: u32,
            signed_int: i32,
            fixed_point: f64,
            number_array: Vec<u8>,
            some_text: String,
            file_descriptor: ::std::os::unix::io::RawFd,
        },
        #[doc = "Only available since version 2 of the interface"]
        GetSecondary {},
        #[doc = "Only available since version 3 of the interface"]
        GetTertiary {},
        #[doc = "link a secondary and a tertiary\n\n\n\nOnly available since version 3 of the interface"]
        Link { sec: super::secondary::Secondary, ter: Option<super::tertiary::Tertiary>, time: u32 },
        #[doc = "This is a destructor, once sent this object cannot be used any longer.\nOnly available since version 4 of the interface"]
        Destroy,
    }
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Event {
        #[doc = "an event with every possible non-object arg"]
        ManyArgsEvt {
            unsigned_int: u32,
            signed_int: i32,
            fixed_point: f64,
            number_array: Vec<u8>,
            some_text: String,
            file_descriptor: ::std::os::unix::io::RawFd,
        },
        #[doc = "acking the creation of a secondary"]
        AckSecondary { sec: super::secondary::Secondary },
        #[doc = "create a new quad optionally replacing a previous one"]
        CycleQuad { new_quad: super::quad::Quad, old_quad: Option<super::quad::Quad> },
    }
    #[derive(Debug, Clone)]
    pub struct TestGlobal {
        id: ObjectId,
        version: u32,
        data: Option<Arc<dyn ObjectData>>,
    }
    impl std::cmp::PartialEq for TestGlobal {
        fn eq(&self, other: &TestGlobal) -> bool {
            self.id == other.id
        }
    }
    impl std::cmp::Eq for TestGlobal {}
    impl super::wayland_client::Proxy for TestGlobal {
        type Request = Request;
        type Event = Event;
        #[inline]
        fn interface() -> &'static Interface {
            &super::TEST_GLOBAL_INTERFACE
        }
        #[inline]
        fn id(&self) -> ObjectId {
            self.id.clone()
        }
        #[inline]
        fn version(&self) -> u32 {
            self.version
        }
        #[inline]
        fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
            self.data
                .as_ref()
                .and_then(|arc| (&**arc).downcast_ref::<QueueProxyData<Self, U>>())
                .map(|data| &data.udata)
        }
        #[inline]
        fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId> {
            if !same_interface(id.interface(), Self::interface()) && !id.is_null() {
                return Err(InvalidId);
            }
            let version = cx.object_info(id.clone()).map(|info| info.version).unwrap_or(0);
            let data = cx.get_object_data(id.clone()).ok();
            Ok(TestGlobal { id, data, version })
        }
        fn parse_event(
            cx: &mut ConnectionHandle,
            msg: Message<ObjectId>,
        ) -> Result<(Self, Self::Event), DispatchError> {
            let me = Self::from_id(cx, msg.sender_id.clone()).unwrap();
            match msg.opcode {
                0u16 => {
                    if let [Argument::Uint(unsigned_int), Argument::Int(signed_int), Argument::Fixed(fixed_point), Argument::Array(number_array), Argument::Str(some_text), Argument::Fd(file_descriptor)] =
                        &msg.args[..]
                    {
                        Ok((
                            me,
                            Event::ManyArgsEvt {
                                unsigned_int: *unsigned_int,
                                signed_int: *signed_int,
                                fixed_point: (*fixed_point as f64) / 256.,
                                number_array: *number_array.clone(),
                                some_text: String::from_utf8_lossy(some_text.as_bytes())
                                    .into_owned(),
                                file_descriptor: *file_descriptor,
                            },
                        ))
                    } else {
                        Err(DispatchError::BadMessage { msg, interface: Self::interface().name })
                    }
                }
                1u16 => {
                    if let [Argument::Object(sec)] = &msg.args[..] {
                        Ok((
                            me,
                            Event::AckSecondary {
                                sec: match <super::secondary::Secondary as Proxy>::from_id(
                                    cx,
                                    sec.clone(),
                                ) {
                                    Ok(p) => p,
                                    Err(_) => {
                                        return Err(DispatchError::BadMessage {
                                            msg,
                                            interface: Self::interface().name,
                                        })
                                    }
                                },
                            },
                        ))
                    } else {
                        Err(DispatchError::BadMessage { msg, interface: Self::interface().name })
                    }
                }
                2u16 => {
                    if let [Argument::NewId(new_quad), Argument::Object(old_quad)] = &msg.args[..] {
                        Ok((
                            me,
                            Event::CycleQuad {
                                new_quad: match <super::quad::Quad as Proxy>::from_id(
                                    cx,
                                    new_quad.clone(),
                                ) {
                                    Ok(p) => p,
                                    Err(_) => {
                                        return Err(DispatchError::BadMessage {
                                            msg,
                                            interface: Self::interface().name,
                                        })
                                    }
                                },
                                old_quad: if old_quad.is_null() {
                                    None
                                } else {
                                    Some(
                                        match <super::quad::Quad as Proxy>::from_id(
                                            cx,
                                            old_quad.clone(),
                                        ) {
                                            Ok(p) => p,
                                            Err(_) => {
                                                return Err(DispatchError::BadMessage {
                                                    msg,
                                                    interface: Self::interface().name,
                                                })
                                            }
                                        },
                                    )
                                },
                            },
                        ))
                    } else {
                        Err(DispatchError::BadMessage { msg, interface: Self::interface().name })
                    }
                }
                _ => Err(DispatchError::BadMessage { msg, interface: Self::interface().name }),
            }
        }
        fn write_request(
            &self,
            cx: &mut ConnectionHandle,
            msg: Self::Request,
        ) -> Result<Message<ObjectId>, InvalidId> {
            match msg {
                Request::ManyArgs {
                    unsigned_int,
                    signed_int,
                    fixed_point,
                    number_array,
                    some_text,
                    file_descriptor,
                } => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 0u16,
                    args: smallvec::smallvec![
                        Argument::Uint(unsigned_int),
                        Argument::Int(signed_int),
                        Argument::Fixed((fixed_point * 256.) as i32),
                        Argument::Array(Box::new(number_array)),
                        Argument::Str(Box::new(std::ffi::CString::new(some_text).unwrap())),
                        Argument::Fd(file_descriptor)
                    ],
                }),
                Request::GetSecondary {} => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 1u16,
                    args: smallvec::smallvec![{
                        let my_info = cx.object_info(self.id())?;
                        let placeholder = cx.placeholder_id(Some((
                            super::secondary::Secondary::interface(),
                            my_info.version,
                        )));
                        Argument::NewId(placeholder)
                    }],
                }),
                Request::GetTertiary {} => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 2u16,
                    args: smallvec::smallvec![{
                        let my_info = cx.object_info(self.id())?;
                        let placeholder = cx.placeholder_id(Some((
                            super::tertiary::Tertiary::interface(),
                            my_info.version,
                        )));
                        Argument::NewId(placeholder)
                    }],
                }),
                Request::Link { sec, ter, time } => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 3u16,
                    args: smallvec::smallvec![
                        Argument::Object(Proxy::id(&sec)),
                        if let Some(obj) = ter {
                            Argument::Object(Proxy::id(&obj))
                        } else {
                            Argument::Object(cx.null_id())
                        },
                        Argument::Uint(time)
                    ],
                }),
                Request::Destroy {} => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 4u16,
                    args: smallvec::smallvec![],
                }),
            }
        }
    }
    impl TestGlobal {
        #[allow(clippy::too_many_arguments)]
        pub fn many_args(
            &self,
            cx: &mut ConnectionHandle,
            unsigned_int: u32,
            signed_int: i32,
            fixed_point: f64,
            number_array: Vec<u8>,
            some_text: String,
            file_descriptor: ::std::os::unix::io::RawFd,
        ) {
            let _ = cx.send_request(
                self,
                Request::ManyArgs {
                    unsigned_int,
                    signed_int,
                    fixed_point,
                    number_array,
                    some_text,
                    file_descriptor,
                },
                None,
            );
        }
        #[allow(clippy::too_many_arguments)]
        pub fn get_secondary<D: Dispatch<super::secondary::Secondary> + 'static>(
            &self,
            cx: &mut ConnectionHandle,
            qh: &QueueHandle<D>,
            udata: <D as Dispatch<super::secondary::Secondary>>::UserData,
        ) -> Result<super::secondary::Secondary, InvalidId> {
            let ret = cx.send_request(
                self,
                Request::GetSecondary {},
                Some(qh.make_data::<super::secondary::Secondary>(udata)),
            )?;
            Proxy::from_id(cx, ret)
        }
        #[allow(clippy::too_many_arguments)]
        pub fn get_tertiary<D: Dispatch<super::tertiary::Tertiary> + 'static>(
            &self,
            cx: &mut ConnectionHandle,
            qh: &QueueHandle<D>,
            udata: <D as Dispatch<super::tertiary::Tertiary>>::UserData,
        ) -> Result<super::tertiary::Tertiary, InvalidId> {
            let ret = cx.send_request(
                self,
                Request::GetTertiary {},
                Some(qh.make_data::<super::tertiary::Tertiary>(udata)),
            )?;
            Proxy::from_id(cx, ret)
        }
        #[allow(clippy::too_many_arguments)]
        pub fn link(
            &self,
            cx: &mut ConnectionHandle,
            sec: &super::secondary::Secondary,
            ter: Option<&super::tertiary::Tertiary>,
            time: u32,
        ) {
            let _ = cx.send_request(self, Request::Link { sec: sec.clone(), ter: ter.cloned(), time }, None);
        }
        #[allow(clippy::too_many_arguments)]
        pub fn destroy(&self, cx: &mut ConnectionHandle) {
            let _ = cx.send_request(self, Request::Destroy {}, None);
        }
    }
}
pub mod secondary {
    use super::wayland_client::{
        backend::{
            protocol::{same_interface, Argument, Interface, Message, WEnum},
            smallvec, InvalidId, ObjectData, ObjectId,
        },
        ConnectionHandle, Dispatch, DispatchError, Proxy, QueueHandle, QueueProxyData,
    };
    use std::sync::Arc;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_DESTROY_SINCE: u32 = 2u32;
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Request {
        #[doc = "This is a destructor, once sent this object cannot be used any longer.\nOnly available since version 2 of the interface"]
        Destroy,
    }
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Event {}
    #[derive(Debug, Clone)]
    pub struct Secondary {
        id: ObjectId,
        version: u32,
        data: Option<Arc<dyn ObjectData>>,
    }
    impl std::cmp::PartialEq for Secondary {
        fn eq(&self, other: &Secondary) -> bool {
            self.id == other.id
        }
    }
    impl std::cmp::Eq for Secondary {}
    impl super::wayland_client::Proxy for Secondary {
        type Request = Request;
        type Event = Event;
        #[inline]
        fn interface() -> &'static Interface {
            &super::SECONDARY_INTERFACE
        }
        #[inline]
        fn id(&self) -> ObjectId {
            self.id.clone()
        }
        #[inline]
        fn version(&self) -> u32 {
            self.version
        }
        #[inline]
        fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
            self.data
                .as_ref()
                .and_then(|arc| (&**arc).downcast_ref::<QueueProxyData<Self, U>>())
                .map(|data| &data.udata)
        }
        #[inline]
        fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId> {
            if !same_interface(id.interface(), Self::interface()) && !id.is_null() {
                return Err(InvalidId);
            }
            let version = cx.object_info(id.clone()).map(|info| info.version).unwrap_or(0);
            let data = cx.get_object_data(id.clone()).ok();
            Ok(Secondary { id, data, version })
        }
        fn parse_event(
            cx: &mut ConnectionHandle,
            msg: Message<ObjectId>,
        ) -> Result<(Self, Self::Event), DispatchError> {
            let me = Self::from_id(cx, msg.sender_id.clone()).unwrap();
            match msg.opcode {
                _ => Err(DispatchError::BadMessage { msg, interface: Self::interface().name }),
            }
        }
        fn write_request(
            &self,
            cx: &mut ConnectionHandle,
            msg: Self::Request,
        ) -> Result<Message<ObjectId>, InvalidId> {
            match msg {
                Request::Destroy {} => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 0u16,
                    args: smallvec::smallvec![],
                }),
            }
        }
    }
    impl Secondary {
        #[allow(clippy::too_many_arguments)]
        pub fn destroy(&self, cx: &mut ConnectionHandle) {
            let _ = cx.send_request(self, Request::Destroy {}, None);
        }
    }
}
pub mod tertiary {
    use super::wayland_client::{
        backend::{
            protocol::{same_interface, Argument, Interface, Message, WEnum},
            smallvec, InvalidId, ObjectData, ObjectId,
        },
        ConnectionHandle, Dispatch, DispatchError, Proxy, QueueHandle, QueueProxyData,
    };
    use std::sync::Arc;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_DESTROY_SINCE: u32 = 3u32;
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Request {
        #[doc = "This is a destructor, once sent this object cannot be used any longer.\nOnly available since version 3 of the interface"]
        Destroy,
    }
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Event {}
    #[derive(Debug, Clone)]
    pub struct Tertiary {
        id: ObjectId,
        version: u32,
        data: Option<Arc<dyn ObjectData>>,
    }
    impl std::cmp::PartialEq for Tertiary {
        fn eq(&self, other: &Tertiary) -> bool {
            self.id == other.id
        }
    }
    impl std::cmp::Eq for Tertiary {}
    impl super::wayland_client::Proxy for Tertiary {
        type Request = Request;
        type Event = Event;
        #[inline]
        fn interface() -> &'static Interface {
            &super::TERTIARY_INTERFACE
        }
        #[inline]
        fn id(&self) -> ObjectId {
            self.id.clone()
        }
        #[inline]
        fn version(&self) -> u32 {
            self.version
        }
        #[inline]
        fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
            self.data
                .as_ref()
                .and_then(|arc| (&**arc).downcast_ref::<QueueProxyData<Self, U>>())
                .map(|data| &data.udata)
        }
        #[inline]
        fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId> {
            if !same_interface(id.interface(), Self::interface()) && !id.is_null() {
                return Err(InvalidId);
            }
            let version = cx.object_info(id.clone()).map(|info| info.version).unwrap_or(0);
            let data = cx.get_object_data(id.clone()).ok();
            Ok(Tertiary { id, data, version })
        }
        fn parse_event(
            cx: &mut ConnectionHandle,
            msg: Message<ObjectId>,
        ) -> Result<(Self, Self::Event), DispatchError> {
            let me = Self::from_id(cx, msg.sender_id.clone()).unwrap();
            match msg.opcode {
                _ => Err(DispatchError::BadMessage { msg, interface: Self::interface().name }),
            }
        }
        fn write_request(
            &self,
            cx: &mut ConnectionHandle,
            msg: Self::Request,
        ) -> Result<Message<ObjectId>, InvalidId> {
            match msg {
                Request::Destroy {} => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 0u16,
                    args: smallvec::smallvec![],
                }),
            }
        }
    }
    impl Tertiary {
        #[allow(clippy::too_many_arguments)]
        pub fn destroy(&self, cx: &mut ConnectionHandle) {
            let _ = cx.send_request(self, Request::Destroy {}, None);
        }
    }
}
pub mod quad {
    use super::wayland_client::{
        backend::{
            protocol::{same_interface, Argument, Interface, Message, WEnum},
            smallvec, InvalidId, ObjectData, ObjectId,
        },
        ConnectionHandle, Dispatch, DispatchError, Proxy, QueueHandle, QueueProxyData,
    };
    use std::sync::Arc;
    #[doc = r" The minimal object version supporting this request"]
    pub const REQ_DESTROY_SINCE: u32 = 3u32;
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Request {
        #[doc = "This is a destructor, once sent this object cannot be used any longer.\nOnly available since version 3 of the interface"]
        Destroy,
    }
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Event {}
    #[derive(Debug, Clone)]
    pub struct Quad {
        id: ObjectId,
        version: u32,
        data: Option<Arc<dyn ObjectData>>,
    }
    impl std::cmp::PartialEq for Quad {
        fn eq(&self, other: &Quad) -> bool {
            self.id == other.id
        }
    }
    impl std::cmp::Eq for Quad {}
    impl super::wayland_client::Proxy for Quad {
        type Request = Request;
        type Event = Event;
        #[inline]
        fn interface() -> &'static Interface {
            &super::QUAD_INTERFACE
        }
        #[inline]
        fn id(&self) -> ObjectId {
            self.id.clone()
        }
        #[inline]
        fn version(&self) -> u32 {
            self.version
        }
        #[inline]
        fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
            self.data
                .as_ref()
                .and_then(|arc| (&**arc).downcast_ref::<QueueProxyData<Self, U>>())
                .map(|data| &data.udata)
        }
        #[inline]
        fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId> {
            if !same_interface(id.interface(), Self::interface()) && !id.is_null() {
                return Err(InvalidId);
            }
            let version = cx.object_info(id.clone()).map(|info| info.version).unwrap_or(0);
            let data = cx.get_object_data(id.clone()).ok();
            Ok(Quad { id, data, version })
        }
        fn parse_event(
            cx: &mut ConnectionHandle,
            msg: Message<ObjectId>,
        ) -> Result<(Self, Self::Event), DispatchError> {
            let me = Self::from_id(cx, msg.sender_id.clone()).unwrap();
            match msg.opcode {
                _ => Err(DispatchError::BadMessage { msg, interface: Self::interface().name }),
            }
        }
        fn write_request(
            &self,
            cx: &mut ConnectionHandle,
            msg: Self::Request,
        ) -> Result<Message<ObjectId>, InvalidId> {
            match msg {
                Request::Destroy {} => Ok(Message {
                    sender_id: self.id.clone(),
                    opcode: 0u16,
                    args: smallvec::smallvec![],
                }),
            }
        }
    }
    impl Quad {
        #[allow(clippy::too_many_arguments)]
        pub fn destroy(&self, cx: &mut ConnectionHandle) {
            let _ = cx.send_request(self, Request::Destroy {}, None);
        }
    }
}
