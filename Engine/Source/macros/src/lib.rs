use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields, FieldsUnnamed, Ident, LitStr,
    parse_macro_input,
};

#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    // Parse the representation
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        Data::Enum(ref data) => derive_event_enum(&input, data),
        Data::Struct(ref data) => derive_event_struct(&input, data),
        _ => panic!("can only derive event from struct or enum"),
    }
}

fn derive_event_enum(input: &DeriveInput, data: &DataEnum) -> proc_macro::TokenStream {
    let name = &input.ident;

    // Generate the match arms for a "message" method
    let arms = data.variants.iter().map(|variant| {
        let var_name = &variant.ident;

        // Look for #[event("foo")]
        let message_format = get_message_format_from_attrs(&variant.attrs); // Default message

        match &variant.fields {
            Fields::Named(fields) => {
                // Get all field names: { system, source, .. }
                let idents = fields.named.iter().map(|f| &f.ident);
                quote! {
                    #[allow(unused_variables)]
                    Self::#var_name { #(#idents,)* .. } => format!(#message_format),
                }
            }
            Fields::Unnamed(fields) => {
                let idents = get_idents_from_unnamed(&message_format, fields);
                quote! {
                    #[allow(unused_variables)]
                    Self::#var_name ( #(#idents,)* .. ) => format!(#message_format),
                }
            }
            Fields::Unit => {
                quote! {
                    #[allow(unused_variables)]
                    Self::#var_name => format!(#message_format),
                }
            }
        }
    });

    let expanded = quote! {
        impl Event for #name {
            fn debug(&self) -> String {
                match self {
                    #(#arms)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn derive_event_struct(input: &DeriveInput, data: &DataStruct) -> proc_macro::TokenStream {
    let name = &input.ident;

    // Look for #[event("foo")]
    let message_format = get_message_format_from_attrs(&input.attrs); // Default message

    let content = match &data.fields {
        Fields::Named(fields) => {
            // Get all field names: { system, source, .. }
            let idents = fields.named.iter().map(|f| &f.ident);
            quote! {
                #[allow(unused_variables)]
                let Self { #(#idents,)* .. } = self;
                format!(#message_format)
            }
        }
        Fields::Unnamed(fields) => {
            let idents = get_idents_from_unnamed(&message_format, fields);
            quote! {
                #[allow(unused_variables)]
                let Self ( #(#idents,)* .. ) = self;
                format!(#message_format, #(#idents),*)
            }
        }
        Fields::Unit => {
            quote! {
                format!(#message_format)
            }
        }
    };

    // Build the output
    let expanded = quote! {
        impl Event for #name {
            fn debug(&self) -> String {
                #content
            }
        }
    };

    // Hand the generated code back to the compiler
    TokenStream::from(expanded)
}

fn get_message_format_from_attrs(attrs: &Vec<Attribute>) -> String {
    for attr in attrs {
        if attr.path().is_ident("event") {
            return attr
                .parse_args::<LitStr>()
                .inspect_err(|e| panic!("parsing event attribute: {e}"))
                .unwrap()
                .value();
        }
    }
    String::from("{self:?}")
}

fn get_idents_from_unnamed(format: &str, fields: &FieldsUnnamed) -> Vec<Ident> {
    // Create the list of potential variables (_0, _1, etc.)
    let all_idents: Vec<_> = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, _)| quote::format_ident!("_{}", i))
        .collect();

    // ONLY include the identifier if "{index}" appears in the message string
    let mut idents = Vec::new();
    for (i, ident) in all_idents.iter().enumerate() {
        let search_pattern = format!("{{{}}}", i); // e.g., "{0}"
        if format.contains(&search_pattern) {
            idents.push(ident.clone());
        }
    }

    idents
}
