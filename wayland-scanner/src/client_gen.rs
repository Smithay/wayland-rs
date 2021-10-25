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

    let parse_body = crate::common::gen_parse_body(interface, Side::Client);
    let write_body = crate::common::gen_write_body(interface, Side::Client);
    let methods = gen_methods(interface);

    quote! {
        #mod_doc
        pub mod #mod_name {
            use std::sync::Arc;

            use super::wayland_client::{
                backend::{smallvec, ObjectData, ObjectId, InvalidId, protocol::{WEnum, Argument, Message, Interface, same_interface}},
                QueueProxyData, Proxy, ConnectionHandle, Dispatch, QueueHandle, DispatchError
            };

            #enums
            #sinces
            #requests
            #events

            #[derive(Debug, Clone)]
            pub struct #iface_name {
                id: ObjectId,
                version: u32,
                data: Option<Arc<dyn ObjectData>>,
            }

            impl std::cmp::PartialEq for #iface_name {
                fn eq(&self, other: &#iface_name) -> bool {
                    self.id == other.id
                }
            }

            impl std::cmp::Eq for #iface_name {}

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
                fn version(&self) -> u32 {
                    self.version
                }

                #[inline]
                fn data<D: Dispatch<#iface_name> + 'static>(&self) -> Option<&<D as Dispatch<#iface_name>>::UserData> {
                    self.data.as_ref().and_then(|arc| (&**arc).downcast_ref::<QueueProxyData<Self, D>>()).map(|data| &data.udata)
                }

                #[inline]
                fn from_id(cx: &mut ConnectionHandle, id: ObjectId) -> Result<Self, InvalidId> {
                    if !same_interface(id.interface(), Self::interface()) {
                        return Err(InvalidId);
                    }
                    let info = cx.object_info(id.clone())?;
                    let data = cx.get_object_data(id.clone()).ok();
                    Ok(#iface_name { id, data, version: info.version })
                }

                fn parse_event(cx: &mut ConnectionHandle, msg: Message<ObjectId>) -> Result<(Self, Self::Event), DispatchError> {
                    #parse_body
                }

                fn write_request(&self, cx: &mut ConnectionHandle, msg: Self::Request) -> Result<Message<ObjectId>, InvalidId> {
                    #write_body
                }
            }

            impl #iface_name {
                #methods
            }
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
                    pub fn #method_name<D: Dispatch<super::#created_iface_mod::#created_iface_type> + 'static>(&self, cx: &mut ConnectionHandle, #(#fn_args,)* qh: &QueueHandle<D>) -> Result<super::#created_iface_mod::#created_iface_type, InvalidId> {
                        let ret = cx.send_request(
                            self,
                            Request::#enum_variant {
                                #(#enum_args),*
                            },
                            Some(qh.make_data::<super::#created_iface_mod::#created_iface_type>())
                        )?;
                        Proxy::from_id(cx, ret)
                    }
                }
            },
            Some(None) => {
                // a bind-like request
                quote! {
                    #[allow(clippy::too_many_arguments)]
                    pub fn #method_name<I: Proxy + 'static, D: Dispatch<I> + 'static>(&self, cx: &mut ConnectionHandle, #(#fn_args,)* qh: &QueueHandle<D>) -> Result<I, InvalidId> {
                        let placeholder = cx.placeholder_id(Some((I::interface(), version)));
                        let ret = cx.send_request(
                            self,
                            Request::#enum_variant {
                                #(#enum_args),*
                            },
                            Some(qh.make_data::<I>())
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
