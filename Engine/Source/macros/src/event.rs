use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DataEnum, DataStruct, DeriveInput, Fields, FieldsUnnamed, Ident, LitStr};

pub fn derive_event_enum(input: &DeriveInput, data: &DataEnum) -> syn::Result<TokenStream> {
    let name = &input.ident;
    // Split generics into the three parts needed for an impl block:
    // <T: Any>  |  <T>  |  where T: Any
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let content = if data.variants.is_empty() {
        // Nothing to match on; fall back to the Debug representation.
        quote! { format!("{self:?}") }
    } else {
        let arms = data
            .variants
            .iter()
            .map(|variant| {
                let var_name = &variant.ident;
                let fmt = get_message_format_from_attrs(&variant.attrs)?;
                let (pattern, format_expr) = create_field_bindings(&variant.fields, &fmt);
                Ok(quote! {
                    #[allow(unused_variables)]
                    Self::#var_name #pattern => #format_expr,
                })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        quote! { match self { #(#arms)* } }
    };

    Ok(quote! {
        impl #impl_generics Event for #name #ty_generics #where_clause {
            fn debug(&self) -> String {
                #content
            }
        }
    })
}

pub fn derive_event_struct(input: &DeriveInput, data: &DataStruct) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fmt = get_message_format_from_attrs(&input.attrs)?;
    let (pattern, format_expr) = create_field_bindings(&data.fields, &fmt);

    let content = match data.fields {
        // Unit structs need no destructuring; everything else gets a `let Self …`.
        Fields::Unit => format_expr,
        _ => quote! {
            #[allow(unused_variables)]
            let Self #pattern = self;
            #format_expr
        },
    };

    Ok(quote! {
        impl #impl_generics Event for #name #ty_generics #where_clause {
            fn debug(&self) -> String {
                #content
            }
        }
    })
}

/// This will create the field destructuring pattern & the expression for the
/// formatting of the field
/// Returns (pattern, format expression)
fn create_field_bindings(fields: &Fields, message_format: &str) -> (TokenStream, TokenStream) {
    match fields {
        // struct Foo { x: T, y: T }  →  let Self { x, y, .. } = self;
        Fields::Named(named) => {
            let idents: Vec<_> = named.named.iter().map(|f| &f.ident).collect();
            // `..` lets the pattern survive the addition of new fields.
            let pattern = quote! { { #(#idents,)* .. } };
            let format_expr = quote! { format!(#message_format) };

            (pattern, format_expr)
        }
        // struct Foo(T, T)  →  let Self(_0, _1) = self;
        Fields::Unnamed(unnamed) => {
            let (fmt, idents) = rewrite_unnamed_placeholders(message_format, unnamed);
            // No `..`: we enumerate every field, so the pattern is already
            // exhaustive. Adding `..` would cause an "unnecessary `..`" lint.
            let pattern = quote! { ( #(#idents,)* ) };
            let format_expr = quote! { format!(#fmt) };

            (pattern, format_expr)
        }
        // struct Foo;  →  nothing to destructure.
        Fields::Unit => (quote! {}, quote! { format!(#message_format) }),
    }
}

/// Extracts the format string from the first `#[event("…")]` attribute found.
///
/// Returns `"{self:?}"` when no `#[event]` attribute is present.
/// Emits a `syn::Error` with a proper source span if the attribute argument
/// is not a string literal.
fn get_message_format_from_attrs(attrs: &[Attribute]) -> syn::Result<String> {
    for attr in attrs {
        if attr.path().is_ident("event") {
            return Ok(attr
                .parse_args::<LitStr>()
                .map_err(|_| {
                    syn::Error::new_spanned(attr, "expected a string literal: #[event(\"…\")]")
                })?
                .value());
        }
    }
    Ok(String::from("{self:?}"))
}

/// Rewrites positional `{0}`, `{1}`, … placeholders to the named bindings
/// `{_0}`, `{_1}`, … used when destructuring unnamed (tuple) fields, and
/// returns the rewritten format string together with the binding [`Ident`]s.
fn rewrite_unnamed_placeholders(format: &str, fields: &FieldsUnnamed) -> (String, Vec<Ident>) {
    // Create the list of potential variables (_0, _1, etc.)
    let idents: Vec<Ident> = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, _)| quote::format_ident!("_{}", i))
        .collect();

    let new_format = idents
        .iter()
        .enumerate()
        .fold(format.to_string(), |acc, (i, _)| {
            acc.replace(&format!("{{{i}}}"), &format!("{{_{i}}}"))
        });

    (new_format, idents)
}
