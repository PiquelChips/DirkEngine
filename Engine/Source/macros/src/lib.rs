use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

mod event;

#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    // Parse the representation
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        Data::Enum(ref data) => event::derive_event_enum(&input, data),
        Data::Struct(ref data) => event::derive_event_struct(&input, data),
        _ => panic!("can only derive event from struct or enum"),
    }
}

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    // Split generics into the three parts needed for an impl block:
    // <T: Any>  |  <T>  |  where T: Any
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics Component for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}
