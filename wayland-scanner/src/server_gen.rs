use proc_macro2::{Ident, Span, TokenStream};

use quote::{format_ident, quote};

use crate::{
    protocol::{Interface, Protocol, Type},
    util::{description_to_doc_attr, dotted_to_relname, is_keyword, snake_to_camel, to_doc_attr},
    Side,
};

pub fn generate_server_objects(protocol: &Protocol) -> TokenStream {
    protocol
        .interfaces
        .iter()
        .filter(|iface| iface.name != "wl_display" && iface.name != "wl_registry")
        .map(generate_objects_for)
        .collect()
}

fn generate_objects_for(interface: &Interface) -> TokenStream {
    let mod_name = Ident::new(&interface.name, Span::call_site());
    let mod_doc = interface.description.as_ref().map(description_to_doc_attr);
    let iface_name = Ident::new(&snake_to_camel(&interface.name), Span::call_site());
    let iface_const_name = format_ident!("{}_INTERFACE", interface.name.to_ascii_uppercase());

    let enums = crate::common::generate_enums_for(interface);
    let msg_constants = crate::common::gen_msg_constants(&interface.requests, &interface.events);

    let requests = crate::common::gen_message_enum(
        &format_ident!("Request"),
        Side::Server,
        true,
        &interface.requests,
    );
    let events = crate::common::gen_message_enum(
        &format_ident!("Event"),
        Side::Server,
        false,
        &interface.events,
    );

    let parse_body = crate::common::gen_parse_body(interface, Side::Server);
    let write_body = crate::common::gen_write_body(interface, Side::Server);
    let methods = gen_methods(interface);

    let event_ref = if interface.requests.is_empty() {
        "This interface has no requests."
    } else {
        "See also the [Request] enum for this interface."
    };
    let docs = match &interface.description {
        Some((short, long)) => format!("{}\n\n{}\n\n{}", short, long, event_ref),
        None => format!("{}\n\n{}", interface.name, event_ref),
    };
    let doc_attr = to_doc_attr(&docs);

    quote! {
        #mod_doc
        pub mod #mod_name {
            use std::sync::Arc;

            use super::wayland_server::{
                backend::{
                    smallvec, ObjectData, ObjectId, InvalidId, io_lifetimes, WeakHandle,
                    protocol::{WEnum, Argument, Message, Interface, same_interface}
                },
                Resource, Dispatch, DisplayHandle, DispatchError, ResourceData, New, Weak,
            };

            #enums
            #msg_constants
            #requests
            #events

            #doc_attr
            #[derive(Debug, Clone)]
            pub struct #iface_name {
                id: ObjectId,
                version: u32,
                data: Option<Arc<dyn std::any::Any + Send + Sync + 'static>>,
                handle: WeakHandle,
            }

            impl std::cmp::PartialEq for #iface_name {
                fn eq(&self, other: &#iface_name) -> bool {
                    self.id == other.id
                }
            }

            impl std::cmp::Eq for #iface_name {}

            impl PartialEq<Weak<#iface_name>> for #iface_name {
                fn eq(&self, other: &Weak<#iface_name>) -> bool {
                    self.id == other.id()
                }
            }

            impl std::borrow::Borrow<ObjectId> for #iface_name {
                fn borrow(&self) -> &ObjectId {
                    &self.id
                }
            }

            impl std::hash::Hash for #iface_name {
                fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                    self.id.hash(state)
                }
            }

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
                fn data<U: 'static>(&self) -> Option<&U> {
                    self.data.as_ref().and_then(|arc| (&**arc).downcast_ref::<ResourceData<Self, U>>()).map(|data| &data.udata)
                }

                #[inline]
                fn object_data(&self) -> Option<&Arc<dyn std::any::Any + Send + Sync>> {
                    self.data.as_ref()
                }

                fn handle(&self) -> &WeakHandle {
                    &self.handle
                }

                #[inline]
                fn from_id(conn: &DisplayHandle, id: ObjectId) -> Result<Self, InvalidId> {
                    if !same_interface(id.interface(), Self::interface()) && !id.is_null(){
                        return Err(InvalidId)
                    }
                    let version = conn.object_info(id.clone()).map(|info| info.version).unwrap_or(0);
                    let data = conn.get_object_data(id.clone()).ok();
                    Ok(#iface_name { id, data, version, handle: conn.backend_handle().downgrade() })
                }

                fn send_event(&self, evt: Self::Event) -> Result<(), InvalidId> {
                    let handle = DisplayHandle::from(self.handle.upgrade().ok_or(InvalidId)?);
                    handle.send_event(self, evt)
                }

                fn parse_request(conn: &DisplayHandle, msg: Message<ObjectId, io_lifetimes::OwnedFd>) -> Result<(Self, Self::Request), DispatchError> {
                    #parse_body
                }

                fn write_event(&self, conn: &DisplayHandle, msg: Self::Event) -> Result<Message<ObjectId, std::os::unix::io::RawFd>, InvalidId> {
                    #write_body
                }

                fn __set_object_data(&mut self, odata: std::sync::Arc<dyn std::any::Any + Send + Sync + 'static>) {
                    self.data = Some(odata);
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
            let method_name = format_ident!(
                "{}{}",
                if is_keyword(&request.name) { "_" } else { "" },
                request.name
            );
            let enum_variant = Ident::new(&snake_to_camel(&request.name), Span::call_site());

            let fn_args = request.args.iter().flat_map(|arg| {
                let arg_name =
                    format_ident!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name);

                let arg_type = if let Some(ref enu) = arg.enum_ {
                    let enum_type = dotted_to_relname(enu);
                    quote! { #enum_type }
                } else {
                    match arg.typ {
                        Type::Uint => quote! { u32 },
                        Type::Int => quote! { i32 },
                        Type::Fixed => quote! { f64 },
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
                        Type::Fd => quote! { ::std::os::unix::io::RawFd },
                        Type::Object | Type::NewId => {
                            let iface = arg.interface.as_ref().unwrap();
                            let iface_mod = Ident::new(iface, Span::call_site());
                            let iface_type = Ident::new(&snake_to_camel(iface), Span::call_site());
                            if arg.allow_null {
                                quote! { Option<&super::#iface_mod::#iface_type> }
                            } else {
                                quote! { &super::#iface_mod::#iface_type }
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
                let arg_name =
                    format_ident!("{}{}", if is_keyword(&arg.name) { "_" } else { "" }, arg.name);
                if arg.enum_.is_some() {
                    Some(quote! { #arg_name: WEnum::Value(#arg_name) })
                } else if arg.typ == Type::Object || arg.typ == Type::NewId {
                    if arg.allow_null {
                        Some(quote! { #arg_name: #arg_name.cloned() })
                    } else {
                        Some(quote! { #arg_name: #arg_name.clone() })
                    }
                } else {
                    Some(quote! { #arg_name })
                }
            });

            let doc_attr = request.description.as_ref().map(description_to_doc_attr);

            quote! {
                #doc_attr
                #[allow(clippy::too_many_arguments)]
                pub fn #method_name(&self, #(#fn_args),*) {
                    let _ = self.send_event(
                        Event::#enum_variant {
                            #(#enum_args),*
                        }
                    );
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn server_gen() {
        let protocol_file =
            std::fs::File::open("./tests/scanner_assets/test-protocol.xml").unwrap();
        let protocol_parsed = crate::parse::parse(protocol_file);
        let generated: String = super::generate_server_objects(&protocol_parsed).to_string();
        let generated = crate::format_rust_code(&generated);

        let reference =
            std::fs::read_to_string("./tests/scanner_assets/test-server-code.rs").unwrap();
        let reference = crate::format_rust_code(&reference);

        if reference != generated {
            let diff = similar::TextDiff::from_lines(&reference, &generated);
            print!("{}", diff.unified_diff().context_radius(10).header("reference", "generated"));
            panic!("Generated does not match reference!")
        }
    }
}
