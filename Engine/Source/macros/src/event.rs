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

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{DeriveInput, parse_str};

    // ── rewrite_unnamed_placeholders ──────────────────────────────────────────

    fn unnamed_fields_from(src: &str) -> FieldsUnnamed {
        let input: DeriveInput = parse_str(src).unwrap();
        match input.data {
            syn::Data::Struct(s) => match s.fields {
                Fields::Unnamed(u) => u,
                _ => panic!("expected unnamed fields"),
            },
            _ => panic!("expected a struct"),
        }
    }

    #[test]
    fn rewrite_replaces_all_positional_placeholders() {
        let fields = unnamed_fields_from("struct Foo(u32, String);");
        let (fmt, idents) = rewrite_unnamed_placeholders("{0} and {1}", &fields);
        assert_eq!(fmt, "{_0} and {_1}");
        assert_eq!(idents.len(), 2);
        assert_eq!(idents[0], quote::format_ident!("_0"));
        assert_eq!(idents[1], quote::format_ident!("_1"));
    }

    #[test]
    fn rewrite_leaves_non_positional_format_intact() {
        let fields = unnamed_fields_from("struct Foo(u32);");
        let (fmt, idents) = rewrite_unnamed_placeholders("no placeholders", &fields);
        assert_eq!(fmt, "no placeholders");
        assert_eq!(idents.len(), 1);
    }

    #[test]
    fn rewrite_handles_partial_placeholder_use() {
        let fields = unnamed_fields_from("struct Foo(u32, String, bool);");
        let (fmt, _) = rewrite_unnamed_placeholders("only {1} matters", &fields);
        assert_eq!(fmt, "only {_1} matters");
    }

    #[test]
    fn rewrite_handles_repeated_placeholder() {
        let fields = unnamed_fields_from("struct Foo(u32, String);");
        let (fmt, _) = rewrite_unnamed_placeholders("{0} then {0} again", &fields);
        assert_eq!(fmt, "{_0} then {_0} again");
    }

    // ── get_message_format_from_attrs ─────────────────────────────────────────

    #[test]
    fn format_falls_back_to_debug_repr_when_no_attr() {
        let input: DeriveInput = parse_str("struct Foo;").unwrap();
        let result = get_message_format_from_attrs(&input.attrs).unwrap();
        assert_eq!(result, "{self:?}");
    }

    #[test]
    fn format_extracts_literal_from_event_attr() {
        let input: DeriveInput = parse_str(r#"#[event("hello world")] struct Foo;"#).unwrap();
        let result = get_message_format_from_attrs(&input.attrs).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn format_returns_error_for_non_string_arg() {
        let input: DeriveInput = parse_str(r#"#[event(42)] struct Foo;"#).unwrap();
        assert!(get_message_format_from_attrs(&input.attrs).is_err());
    }

    #[test]
    fn format_ignores_unrelated_attributes() {
        let input: DeriveInput = parse_str(r#"#[derive(Debug)] struct Foo;"#).unwrap();
        let result = get_message_format_from_attrs(&input.attrs).unwrap();
        assert_eq!(result, "{self:?}");
    }
}
