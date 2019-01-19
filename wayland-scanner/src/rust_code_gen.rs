use proc_macro2::{Ident, Span, TokenStream};

use common_gen::*;
use protocol::*;
use util::*;
use Side;

pub(crate) fn generate_protocol_client(protocol: Protocol) -> TokenStream {
    let modules = protocol.interfaces.iter().map(|iface| {
        let doc_attr = iface.description.as_ref().map(description_to_doc_attr);
        let mod_name = Ident::new(&iface.name, Span::call_site());
        let iface_name = Ident::new(&snake_to_camel(&iface.name), Span::call_site());

        let enums = &iface.enums;
        let requests = gen_messagegroup(
            &Ident::new("Request", Span::call_site()),
            Side::Client,
            false,
            &iface.requests,
            None,
        );
        let events = gen_messagegroup(
            &Ident::new("Event", Span::call_site()),
            Side::Client,
            true,
            &iface.events,
            None,
        );
        let interface = gen_interface(
            &iface_name,
            &iface.name,
            iface.version,
            None,
        );
        let client_methods = gen_client_methods(&iface_name, &iface.requests);
        let sinces = gen_since_constants(&iface.requests, &iface.events);

        quote! {
            #doc_attr
            pub mod #mod_name {
                use super::{
                    Proxy, NewProxy, AnonymousObject, Interface, MessageGroup, MessageDesc, ArgumentType, Object,
                    Message, Argument, ObjectMetadata,
                };

                #(#enums)*
                #requests
                #events
                #interface
                #client_methods
                #sinces
            }
        }
    });

    quote! {
        #(#modules)*
    }
}

pub(crate) fn generate_protocol_server(protocol: Protocol) -> TokenStream {
    let modules = protocol
        .interfaces
        .iter()
        // display and registry are handled specially
        .filter(|iface| iface.name != "wl_display" && iface.name != "wl_registry")
        .map(|iface| {
            let doc_attr = iface.description.as_ref().map(description_to_doc_attr);
            let mod_name = Ident::new(&iface.name, Span::call_site());

            let enums = &iface.enums;
            let requests = gen_messagegroup(
                &Ident::new("Request", Span::call_site()),
                Side::Server,
                true,
                &iface.requests,
                None,
            );
            let events = gen_messagegroup(
                &Ident::new("Event", Span::call_site()),
                Side::Server,
                false,
                &iface.events,
                None,
            );
            let interface = gen_interface(
                &Ident::new(&snake_to_camel(&iface.name), Span::call_site()),
                &iface.name,
                iface.version,
                None,
            );
            let sinces = gen_since_constants(&iface.requests, &iface.events);

            quote! {
                #doc_attr
                pub mod #mod_name {
                    use super::{
                        Resource, NewResource, AnonymousObject, Interface, MessageGroup, MessageDesc,
                        ArgumentType, Object, Message, Argument, ObjectMetadata
                    };

                    #(#enums)*
                    #requests
                    #events
                    #interface
                    #sinces
                }
            }
        });

    quote! {
        #(#modules)*
    }
}
