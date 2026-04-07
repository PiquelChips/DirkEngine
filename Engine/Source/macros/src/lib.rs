use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Event)]
pub fn event_derive_macro(input: TokenStream) -> TokenStream {
    // Parse the representation
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Build the output
    let expanded = quote! {
        impl Event for #name {
            // To add a method in the future, you'd write it here.
            // Example:
            // fn event_type(&self) -> &'static str {
            //     stringify!(#name)
            // }
        }
    };

    // Hand the generated code back to the compiler
    TokenStream::from(expanded)
}
