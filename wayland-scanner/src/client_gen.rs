use proc_macro2::{Ident, Span, TokenStream};

use quote::quote;

use crate::{
    protocol::{Interface, Protocol, Type},
    util::{dotted_to_relname, is_keyword, snake_to_camel},
    Side,
};

pub fn generate_client_objects(protocol: &Protocol) -> TokenStream {
    let tokens = protocol.interfaces.iter().map(generate_objects_for);
    quote!(
        #(#tokens)*
    )
}

fn generate_objects_for(interface: &Interface) -> TokenStream {
    let mod_name = Ident::new(&interface.name, Span::call_site());
    let mod_doc = interface.description.as_ref().map(crate::util::description_to_doc_attr);
    let iface_name = Ident::new(&snake_to_camel(&interface.name), Span::call_site());
    let iface_const_name = Ident::new(
        &format!("{}_INTERFACE", interface.name.to_ascii_uppercase()),
        Span::call_site(),
    );

    let enums = crate::common::generate_enums_for(interface);
    let sinces = crate::common::gen_since_constants(&interface.requests, &interface.events);

    let requests = crate::common::gen_message_enum(
        &Ident::new("Request", Span::call_site()),
        Side::Client,
        false,
        &interface.requests,
    );
    let events = crate::common::gen_message_enum(
        &Ident::new("Event", Span::call_site()),
        Side::Client,
        true,
        &interface.events,
    );

    let parse_body = gen_parse_body(interface);
    let write_body = gen_write_body(interface);
    let methods = gen_methods(interface);

    quote! {
        #mod_doc
        pub mod #mod_name {
            use std::sync::Arc;

            use super::wayland_client::{
                backend::{smallvec, ObjectId, InvalidId, protocol::{WEnum, Argument, Message, Interface, same_interface}},
                proxy_internals::ProxyData,
                Proxy, ConnectionHandle,
            };

            #enums
            #sinces
            #requests
            #events

            #[derive(Debug, Clone)]
            pub struct #iface_name {
                id: ObjectId,
                data: Option<Arc<ProxyData>>,
            }

            impl super::wayland_client::Proxy for #iface_name {
                type Request = Request;
                type Event = Event;

                #[inline]
                fn interface() -> &'static Interface{
                    &super::#iface_const_name
                }

                #[inline]
                fn id(&self) -> ObjectId {
                    self.id.clone()
                }

                #[inline]
                fn data(&self) -> Option<&Arc<ProxyData>> {
                    self.data.as_ref()
                }

                #[inline]
                fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId> {
                    if same_interface(id.interface(), Self::interface()) {
                        let data = cx.get_proxy_data(id.clone()).ok();
                        Ok(#iface_name { id, data })
                    } else {
                        Err(InvalidId)
                    }
                }

                fn parse_event(cx: &mut ConnectionHandle, msg: Message<ObjectId>) -> Result<(Self, Self::Event), Message<ObjectId>> {
                    #parse_body
                }

                fn write_request(&self, cx: &mut ConnectionHandle, request: Self::Request) -> Result<Message<ObjectId>, InvalidId> {
                    #write_body
                }
            }

            impl #iface_name {
                #methods
            }
        }
    }
}

fn gen_parse_body(interface: &Interface) -> TokenStream {
    let match_arms = interface.events.iter().enumerate().map(|(opcode, msg)| {
        let opcode = opcode as u16;
        let event_name = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
        let args_pat = msg.args.iter().map(|arg| {
            let arg_name = Ident::new(
                &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                Span::call_site(),
            );
            match arg.typ {
                Type::Uint => quote!{ Argument::Uint(#arg_name) },
                Type::Int => quote!{ Argument::Int(#arg_name) },
                Type::String => quote!{ Argument::Str(#arg_name) },
                Type::Fixed => quote!{ Argument::Fixed(#arg_name) },
                Type::Array => quote!{ Argument::Array(#arg_name) },
                Type::Object => quote!{ Argument::Object(#arg_name) },
                Type::NewId => quote!{ Argument::NewId(#arg_name) },
                Type::Fd => quote!{ Argument::Fd(#arg_name) },
                Type::Destructor => panic!("Argument {}.{}.{} has type destructor ?!", interface.name, msg.name, arg.name),
            }
        });

        let arg_names = msg.args.iter().map(|arg| {
            let arg_name = Ident::new(
                &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                Span::call_site(),
            );
            if arg.enum_.is_some() {
                quote! { #arg_name: From::from(*#arg_name as u32) }
            } else {
                match arg.typ {
                    Type::Uint | Type::Int | Type::Fd => quote!{ #arg_name: *#arg_name },
                    Type::Fixed => quote!{ #arg_name: (*#arg_name as f64) / 256.},
                    Type::String => {
                        let string_conversion = quote! {
                            String::from_utf8_lossy(#arg_name.as_bytes()).into_owned()
                        };

                        if arg.allow_null {
                            quote! {
                                #arg_name: {
                                    let s = #string_conversion;
                                    if s.len() == 0 { None } else { Some(s) }
                                }
                            }
                        } else {
                            quote! {
                                #arg_name: #string_conversion
                            }
                        }
                    },
                    Type::Object | Type::NewId => {
                        let create_proxy = if arg.interface.is_some() {
                            quote! {
                                match Proxy::from_id(cx, #arg_name.clone()) {
                                    Ok(p) => p,
                                    Err(_) => return Err(msg),
                                }
                            }
                        } else {
                            quote! { #arg_name.clone() }
                        };
                        if arg.allow_null {
                            quote! {
                                #arg_name: if #arg_name.is_null() { None } else { Some(#create_proxy) }
                            }
                        } else {
                            quote! {
                                #arg_name: #create_proxy
                            }
                        }
                    },
                    Type::Array => {
                        if arg.allow_null {
                            quote!(if #arg_name.len() == 0 { None } else { Some(*#arg_name.clone()) })
                        } else {
                            quote!(#arg_name: *#arg_name.clone())
                        }
                    },
                    Type::Destructor => unreachable!(),
                }
            }
        });

        quote! {
            #opcode => {
                if let [#(#args_pat),*] = &msg.args[..] {
                    Ok((me, Event::#event_name { #(#arg_names),* }))
                } else {
                    Err(msg)
                }
            }
        }
    });

    quote! {
        let me = match Self::from_id(cx, msg.sender_id.clone()) {
            Ok(me) => me,
            Err(_) => return Err(msg),
        };
        match msg.opcode {
            #(#match_arms),*
            _ => return Err(msg)
        }
    }
}

fn gen_write_body(interface: &Interface) -> TokenStream {
    let arms = interface.requests.iter().enumerate().map(|(opcode, msg)| {
        let msg_name = Ident::new(&snake_to_camel(&msg.name), Span::call_site());
        let opcode = opcode as u16;
        let arg_names = msg.args.iter().flat_map(|arg| {
            if arg.typ == Type::NewId && arg.interface.is_some() {
                None
            } else {
                Some(Ident::new(
                    &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                    Span::call_site(),
                ))
            }
        });
        let args = msg.args.iter().map(|arg| {
            let arg_name = Ident::new(
                &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                Span::call_site(),
            );

            match arg.typ {
                Type::Int => if arg.enum_.is_some() { quote!{ Argument::Int(Into::<u32>::into(#arg_name) as i32) } } else { quote!{ Argument::Int(#arg_name) } },
                Type::Uint => if arg.enum_.is_some() { quote!{ Argument::Uint(#arg_name.into()) } } else { quote!{ Argument::Uint(#arg_name) } },
                Type::Fd => quote!{ Argument::Fd(#arg_name) },
                Type::Fixed => quote! { Argument::Fixed((#arg_name * 256.) as i32) },
                Type::Object => if arg.allow_null {
                    quote! { if let Some(obj) = #arg_name { Argument::Object(obj.id()) } else { Argument::Object(cx.null_id()) } }
                } else {
                    quote!{ Argument::Object(#arg_name.id()) }
                },
                Type::Array => if arg.allow_null {
                    quote! { if let Some(array) = #arg_name { Argument::Array(Box::new(array)) } else { Argument::Array(Box::new(Vec::new()))}}
                } else {
                    quote! { Argument::Array(Box::new(#arg_name)) }
                },
                Type::String => if arg.allow_null {
                    quote! { if let Some(string) = #arg_name { Argument::Str(Box::new(std::ffi::CString::new(string).unwrap())) } else { Argument::Str(Box::new(std::ffi::CString::new(Vec::new()).unwrap())) }}
                } else {
                    quote! { Argument::Str(Box::new(std::ffi::CString::new(#arg_name).unwrap())) }
                },
                Type::NewId => if let Some(ref created_interface) = arg.interface {
                    let created_iface_mod = Ident::new(created_interface, Span::call_site());
                    let created_iface_type = Ident::new(&snake_to_camel(created_interface), Span::call_site());
                    quote! { {
                        let my_info = cx.object_info(self.id())?;
                        let placeholder = cx.placeholder_id(Some((super::#created_iface_mod::#created_iface_type::interface(), my_info.version)));
                        Argument::NewId(placeholder)
                    } }
                } else {
                    quote! {
                        Argument::Str(Box::new(std::ffi::CString::new(#arg_name.0.name).unwrap())),
                        Argument::Uint(#arg_name.1),
                        Argument::NewId(cx.placeholder_id(Some((#arg_name.0, #arg_name.1))))
                    }
                },
                Type::Destructor => panic!("Argument {}.{}.{} has type destructor ?!", interface.name, msg.name, arg.name),
            }
        });
        quote! {
            Request::#msg_name { #(#arg_names),* } => Ok(Message {
                sender_id: self.id.clone(),
                opcode: #opcode,
                args: smallvec::smallvec![
                    #(#args),*
                ]
            })
        }
    });
    quote! {
        match request {
            #(#arms),*
        }
    }
}

fn gen_methods(interface: &Interface) -> TokenStream {
    interface.requests.iter().map(|request| {
        let created_interface = request.args.iter().find(|arg| arg.typ == Type::NewId).map(|arg| &arg.interface);

        let method_name = Ident::new(
            &format!("{}{}", if is_keyword(&request.name) { "_" } else { "" }, request.name),
            Span::call_site(),
        );
        let enum_variant = Ident::new(&snake_to_camel(&request.name), Span::call_site());

        let fn_args = request.args.iter().flat_map(|arg| {
            if arg.typ == Type::NewId {
                if arg.interface.is_none() {
                    // the new_id argument of a bind-like method
                    // it shoudl expand as a (interface, type) tuple, but the type is already handled by
                    // the prototype type parameter, so just put a version here
                    return Some(quote! { version: u32 });
                } else {
                    // this is a regular new_id, skip it
                    return None;
                }
            }

            let arg_name = Ident::new(
                &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                Span::call_site(),
            );

            let arg_type =  if let Some(ref enu) = arg.enum_ {
                let enum_type = dotted_to_relname(enu);
                quote! { #enum_type }
            } else {
                match arg.typ {
                    Type::Uint => quote!(u32),
                    Type::Int => quote!(i32),
                    Type::Fixed => quote!(f64),
                    Type::String => if arg.allow_null { quote!{ Option<String> } } else { quote!{ String } },
                    Type::Array => if arg.allow_null { quote!{ Option<Vec<u8>> } } else { quote!{ Vec<u8> } },
                    Type::Fd => quote!(::std::os::unix::io::RawFd),
                    Type::Object => {
                        let iface = arg.interface.as_ref().unwrap();
                        let iface_mod = Ident::new(iface, Span::call_site());
                        let iface_type =
                            Ident::new(&snake_to_camel(iface), Span::call_site());
                        if arg.allow_null { quote! { Option<super::#iface_mod::#iface_type> } } else { quote! { super::#iface_mod::#iface_type } }
                    },
                    Type::NewId => unreachable!(),
                    Type::Destructor => panic!("An argument cannot have type \"destructor\"."),
                }
            };

            Some(quote! {
                #arg_name: #arg_type
            })
        });

        let enum_args = request.args.iter().flat_map(|arg| {
            let arg_name = Ident::new(
                &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                Span::call_site(),
            );
            if arg.enum_.is_some() {
                Some(quote! { #arg_name: WEnum::Value(#arg_name) })
            } else if arg.typ == Type::NewId {
                if arg.interface.is_none() {
                    Some(quote! { #arg_name: (I::interface(), version) })
                } else {
                    None
                }
            } else {
                Some(quote! { #arg_name })
            }
        });

        match created_interface {
            Some(Some(ref created_interface)) => {
                // a regular creating request
                let created_iface_mod = Ident::new(created_interface, Span::call_site());
                let created_iface_type = Ident::new(&snake_to_camel(created_interface), Span::call_site());
                quote! {
                    #[allow(clippy::too_many_arguments)]
                    pub fn #method_name(&self, cx: &mut ConnectionHandle, #(#fn_args,)* data: Option<Arc<ProxyData>>) -> Result<super::#created_iface_mod::#created_iface_type, InvalidId> {
                        let ret = cx.send_request(
                            self,
                            Request::#enum_variant {
                                #(#enum_args),*
                            },
                            data
                        )?;
                        Proxy::from_id(cx, ret)
                    }
                }
            },
            Some(None) => {
                // a bind-like request
                quote! {
                    #[allow(clippy::too_many_arguments)]
                    pub fn #method_name<I: Proxy>(&self, cx: &mut ConnectionHandle, #(#fn_args,)* data: Option<Arc<ProxyData>>) -> Result<I, InvalidId> {
                        let placeholder = cx.placeholder_id(Some((I::interface(), version)));
                        let ret = cx.send_request(
                            self,
                            Request::#enum_variant {
                                #(#enum_args),*
                            },
                            data
                        )?;
                        Proxy::from_id(cx, ret)
                    }
                }
            },
            None => {
                // a non-creating request
                quote! {
                    #[allow(clippy::too_many_arguments)]
                    pub fn #method_name(&self, cx: &mut ConnectionHandle, #(#fn_args),*) {
                        let _ = cx.send_request(
                            self,
                            Request::#enum_variant {
                                #(#enum_args),*
                            },
                            None
                        );
                    }
                }
            }
        }
    }).collect()
}
