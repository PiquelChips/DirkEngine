use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemTrait, Pat, TraitItem, TypeParamBound};

pub fn generate_system_code(trait_def: &ItemTrait) -> syn::Result<TokenStream> {
    if !trait_def.supertraits.iter().any(|s| match s {
        TypeParamBound::Trait(t) => t.path.is_ident("System"),
        _ => false,
    }) {
        return Err(syn::Error::new(
            trait_def.ident.span(),
            "this trait must have the `System` trait in its param bounds",
        ));
    }

    let trait_name = &trait_def.ident;
    let any_name = format_ident!("Any{trait_name}");
    let storage = format_ident!("{trait_name}Storage");
    let handle = format_ident!("{trait_name}Handle");

    let fns: Vec<_> = trait_def
        .items
        .iter()
        .filter_map(|item| {
            if let TraitItem::Fn(m) = item {
                Some(m)
            } else {
                None
            }
        })
        .collect();

    let any_sigs = fns.iter().map(|m| {
        let s = &m.sig;
        quote!(#s;)
    });

    let blanket_methods = fns.iter().map(|m| {
        let sig = &m.sig;
        let name = &m.sig.ident;
        let args = get_arg_names_from_sig(&m.sig);
        quote! {
            #sig { T::#name(#(#args),*) }
        }
    });

    Ok(quote! {
        // Re-emit the original trait unchanged.
        #trait_def

        // Any trait
        trait #any_name { #(#any_sigs)* }

        impl<T: #trait_name> #any_name for T { #(#blanket_methods)* }

        // Storage

        // TODO: see about using a slotmap
        #[derive(Default)]
        pub(crate) struct #storage {
            systems: ::std::collections::HashMap<#handle, Box<dyn #any_name>>
        }

        impl #storage {
            pub fn new() -> Self { Self::default() }

            pub fn insert<S: #trait_name>(&mut self, system: S) -> #handle {
                todo!("insert into storage")
            }

            pub fn remove(&mut self, handle: #handle) {
                todo!("remove from storage")
            }

            // TODO: looping over all the traits
        }

        #[derive(Clone, Copy)]
        #[doc = concat!("Handle type for [`", stringify!(#trait_name), "`]")]
        #[doc = "This allows subsystems to have a reference to their systems
        in case they need to remove them."]
        pub struct #handle(u32);
    })
}

fn get_arg_names_from_sig(sig: &syn::Signature) -> Vec<TokenStream> {
    sig.inputs
        .iter()
        .map(|arg| match arg {
            FnArg::Typed(pt) => match &*pt.pat {
                Pat::Ident(id) => quote! { #id },
                &_ => todo!("implement other pattern types"),
            },
            FnArg::Receiver(_) => quote! {self},
        })
        .collect()
}
