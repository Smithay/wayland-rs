use proc_macro2::{Ident, Span, TokenStream};

use quote::quote;

use crate::{
    protocol::{Interface, Protocol},
    util::{description_to_doc_attr, snake_to_camel},
};

pub fn generate_aliases(protocol: &Protocol) -> TokenStream {
    protocol.interfaces.iter().map(generate_aliases_for).collect()
}

fn generate_aliases_for(interface: &Interface) -> TokenStream {
    let zname = &interface.name;
    let name = interface.name.strip_prefix('z').unwrap_or(zname);

    let zmod_name = Ident::new(zname, Span::call_site());
    let zmod = quote!(super::#zmod_name);

    let ziface_name = Ident::new(&snake_to_camel(zname), Span::call_site());
    let iface_name = Ident::new(&snake_to_camel(name), Span::call_site());

    let mod_name = Ident::new(name, Span::call_site());
    let mod_doc = interface.description.as_ref().map(description_to_doc_attr);

    let enums = crate::common::generate_enum_reexports_for(&zmod, interface);
    let sinces =
        crate::common::gen_msg_constant_reexports(&zmod, &interface.requests, &interface.events);

    quote! {
        #mod_doc
        pub mod #mod_name {
            #enums
            #sinces
            pub use #zmod::Event;
            pub use #zmod::Request;
            pub use #zmod::#ziface_name as #iface_name;
        }
    }
}
