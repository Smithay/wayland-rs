use proc_macro2::{Ident, Span, TokenStream};

use quote::quote;

use crate::{
    protocol::{Interface, Protocol, Type},
    util::{dotted_to_relname, is_keyword, snake_to_camel},
    Side,
};

pub fn generate_server_objects(protocol: &Protocol) -> TokenStream {
    let tokens = protocol
        .interfaces
        .iter()
        .filter(|iface| iface.name != "wl_display" && iface.name != "wl_registry")
        .map(generate_objects_for);
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
        Side::Server,
        true,
        &interface.requests,
    );
    let events = crate::common::gen_message_enum(
        &Ident::new("Event", Span::call_site()),
        Side::Server,
        false,
        &interface.events,
    );

    let parse_body = crate::common::gen_parse_body(interface, Side::Server);
    let write_body = crate::common::gen_write_body(interface, Side::Server);
    let methods = gen_methods(interface);

    quote! {
        #mod_doc
        pub mod #mod_name {
            use std::sync::Arc;

            use super::wayland_server::{
                backend::{smallvec, ObjectData, ObjectId, InvalidId, protocol::{WEnum, Argument, Message, Interface, same_interface}},
                Resource, Dispatch, DisplayHandle, DispatchError,
            };

            #enums
            #sinces
            #requests
            #events

            #[derive(Debug, Clone)]
            pub struct #iface_name {
                id: ObjectId,
                version: u32,
                data: Option<Arc<dyn std::any::Any + Send + Sync + 'static>>,
            }

            impl std::cmp::PartialEq for #iface_name {
                fn eq(&self, other: &#iface_name) -> bool {
                    self.id == other.id
                }
            }

            impl std::cmp::Eq for #iface_name {}

            impl super::wayland_server::Resource for #iface_name {
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
                    //self.data.as_ref().and_then(|arc| (&**arc).downcast_ref::<QueueProxyData<Self, D>>()).map(|data| &data.udata)
                    unimplemented!()
                }

                #[inline]
                fn from_id<D>(cx: &mut DisplayHandle<D>, id: ObjectId) -> Result<Self, InvalidId> {
                    if !same_interface(id.interface(), Self::interface()) {
                        return Err(InvalidId)
                    }
                    let info = cx.object_info(id.clone())?;
                    let data = cx.get_object_data(id.clone()).ok().map(|udata| udata.into_any_arc());
                    Ok(#iface_name { id, data, version: info.version })
                }

                fn parse_request<D>(cx: &mut DisplayHandle<D>, msg: Message<ObjectId>) -> Result<(Self, Self::Request), DispatchError> {
                    #parse_body
                }

                fn write_event<D>(&self, cx: &mut DisplayHandle<D>, msg: Self::Event) -> Result<Message<ObjectId>, InvalidId> {
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
    interface
        .events
        .iter()
        .map(|request| {
            let method_name = Ident::new(
                &format!("{}{}", if is_keyword(&request.name) { "_" } else { "" }, request.name),
                Span::call_site(),
            );
            let enum_variant = Ident::new(&snake_to_camel(&request.name), Span::call_site());

            let fn_args = request.args.iter().flat_map(|arg| {
                let arg_name = Ident::new(
                    &format!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name),
                    Span::call_site(),
                );

                let arg_type = if let Some(ref enu) = arg.enum_ {
                    let enum_type = dotted_to_relname(enu);
                    quote! { #enum_type }
                } else {
                    match arg.typ {
                        Type::Uint => quote!(u32),
                        Type::Int => quote!(i32),
                        Type::Fixed => quote!(f64),
                        Type::String => {
                            if arg.allow_null {
                                quote! { Option<String> }
                            } else {
                                quote! { String }
                            }
                        }
                        Type::Array => {
                            if arg.allow_null {
                                quote! { Option<Vec<u8>> }
                            } else {
                                quote! { Vec<u8> }
                            }
                        }
                        Type::Fd => quote!(::std::os::unix::io::RawFd),
                        Type::Object | Type::NewId => {
                            let iface = arg.interface.as_ref().unwrap();
                            let iface_mod = Ident::new(iface, Span::call_site());
                            let iface_type = Ident::new(&snake_to_camel(iface), Span::call_site());
                            if arg.allow_null {
                                quote! { Option<super::#iface_mod::#iface_type> }
                            } else {
                                quote! { super::#iface_mod::#iface_type }
                            }
                        }
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
                } else {
                    Some(quote! { #arg_name })
                }
            });

            quote! {
                #[allow(clippy::too_many_arguments)]
                pub fn #method_name<D>(&self, cx: &mut DisplayHandle<D>, #(#fn_args),*) {
                    let _ = cx.send_event(
                        self,
                        Event::#enum_variant {
                            #(#enum_args),*
                        }
                    );
                }
            }
        })
        .collect()
}
