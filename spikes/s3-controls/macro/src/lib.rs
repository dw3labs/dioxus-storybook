//! S3 spike: `#[derive(Controls)]` and `#[derive(ControlEnum)]`.
//!
//! The claim under test (plan problem P2): a proc macro sees field names,
//! types and doc comments, so it can produce everything react-docgen extracts
//! — but type-checked, because the applier it emits must compile.

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Fields, Type, parse_macro_input};

/// Collect the text of `///` doc comments on a field or item.
fn docs_of(attrs: &[syn::Attribute]) -> String {
    let mut out = Vec::new();
    for a in attrs {
        if !a.path().is_ident("doc") { continue; }
        if let syn::Meta::NameValue(nv) = &a.meta {
            if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                out.push(s.value().trim().to_string());
            }
        }
    }
    out.join(" ").trim().to_string()
}

/// The outermost path segment of a type, e.g. `Option<T>` -> "Option".
fn head(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        Type::Reference(r) => head(&r.elem),
        _ => None,
    }
}

/// The `T` in `Option<T>`.
fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(p) = ty else { return None };
    let seg = p.path.segments.last()?;
    if seg.ident != "Option" { return None; }
    let syn::PathArguments::AngleBracketed(a) = &seg.arguments else { return None };
    a.args.iter().find_map(|g| if let syn::GenericArgument::Type(t) = g { Some(t) } else { None })
}

/// Parse `#[control(...)]` into an explicit override, if present.
enum Override { Skip, Text, Number, Color, Select, Radio, Range(f64, f64, f64) }

fn control_attr(attrs: &[syn::Attribute]) -> Option<Override> {
    let a = attrs.iter().find(|a| a.path().is_ident("control"))?;
    let mut found = None;
    let _ = a.parse_nested_meta(|m| {
        let id = m.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
        match id.as_str() {
            "skip"   => found = Some(Override::Skip),
            "text"   => found = Some(Override::Text),
            "number" => found = Some(Override::Number),
            "color"  => found = Some(Override::Color),
            "select" => found = Some(Override::Select),
            "radio"  => found = Some(Override::Radio),
            "range"  => {
                let (mut lo, mut hi, mut st) = (0.0f64, 1.0f64, 0.01f64);
                let _ = m.parse_nested_meta(|r| {
                    let key = r.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
                    let v: syn::Lit = r.value()?.parse()?;
                    let n = match v { syn::Lit::Float(f) => f.base10_parse::<f64>().unwrap_or(0.0),
                                      syn::Lit::Int(i)   => i.base10_parse::<f64>().unwrap_or(0.0),
                                      _ => 0.0 };
                    match key.as_str() { "min" => lo = n, "max" => hi = n, "step" => st = n, _ => {} }
                    Ok(())
                });
                found = Some(Override::Range(lo, hi, st));
            }
            _ => {}
        }
        Ok(())
    });
    found
}

/// Syntactic control inference. Deliberately conservative: an unrecognised
/// type becomes `Control::None` (no widget) rather than a compile error, so
/// adding an exotic prop never breaks the build.
fn infer(ty: &Type) -> proc_macro2::TokenStream {
    let base = option_inner(ty).unwrap_or(ty);
    let h = head(base).unwrap_or_default();
    match h.as_str() {
        "String" | "str" => quote!(s3_core::Control::Text),
        "bool" => quote!(s3_core::Control::Toggle),
        "f32" | "f64" | "i8" | "i16" | "i32" | "i64" | "isize"
        | "u8" | "u16" | "u32" | "u64" | "usize" =>
            quote!(s3_core::Control::Number { min: None, max: None, step: None }),
        "EventHandler" | "Callback" => quote!(s3_core::Control::Action),
        "Element" | "VNode" => quote!(s3_core::Control::None),
        _ => quote!(s3_core::Control::Select {
            options: <#base as s3_core::ControlEnum>::VARIANTS
        }),
    }
}

/// Does this field get a live control (and therefore participate in `apply`)?
fn is_dynamic(ty: &Type, ov: &Option<Override>) -> bool {
    if matches!(ov, Some(Override::Skip)) { return false; }
    let base = option_inner(ty).unwrap_or(ty);
    !matches!(head(base).unwrap_or_default().as_str(),
              "EventHandler" | "Callback" | "Element" | "VNode")
}

#[proc_macro_derive(Controls, attributes(control))]
pub fn derive_controls(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let Data::Struct(ds) = &ast.data else {
        return syn::Error::new_spanned(&ast.ident, "Controls only applies to structs")
            .to_compile_error().into();
    };
    let Fields::Named(named) = &ds.fields else {
        return syn::Error::new_spanned(&ast.ident, "Controls needs named fields")
            .to_compile_error().into();
    };

    let mut rows = Vec::new();
    let mut applies = Vec::new();
    let mut seeds = Vec::new();

    for f in &named.named {
        let ident = f.ident.as_ref().unwrap();
        let fname = ident.to_string();
        let ty = &f.ty;
        let ov = control_attr(&f.attrs);
        let docs = docs_of(&f.attrs);
        let ty_str = ty.to_token_stream().to_string().replace(' ', "");
        let required = option_inner(ty).is_none();

        let control = match &ov {
            Some(Override::Skip)   => quote!(s3_core::Control::None),
            Some(Override::Text)   => quote!(s3_core::Control::Text),
            Some(Override::Number) => quote!(s3_core::Control::Number { min: None, max: None, step: None }),
            Some(Override::Color)  => quote!(s3_core::Control::Color),
            Some(Override::Select) => {
                let base = option_inner(ty).unwrap_or(ty);
                quote!(s3_core::Control::Select { options: <#base as s3_core::ControlEnum>::VARIANTS })
            }
            Some(Override::Radio) => {
                let base = option_inner(ty).unwrap_or(ty);
                quote!(s3_core::Control::Radio { options: <#base as s3_core::ControlEnum>::VARIANTS })
            }
            Some(Override::Range(lo, hi, st)) =>
                quote!(s3_core::Control::Range { min: #lo, max: #hi, step: #st }),
            None => infer(ty),
        };

        rows.push(quote! {
            s3_core::ArgType {
                name: #fname, ty: #ty_str, docs: #docs,
                control: #control, required: #required,
            }
        });

        if is_dynamic(ty, &ov) {
            applies.push(quote! { #ident: args.get_or(#fname, self.#ident.clone()) });
            seeds.push(quote! { m.0.insert(#fname.to_string(), s3_core::ToArg::to_arg(&self.#ident)); });
        } else {
            applies.push(quote! { #ident: self.#ident.clone() });
        }
    }

    quote! {
        impl s3_core::Controllable for #name {
            fn arg_types() -> &'static [s3_core::ArgType] {
                // A `const` block: the whole props table is materialised at
                // compile time, zero runtime cost.
                const ROWS: &[s3_core::ArgType] = &[ #(#rows),* ];
                ROWS
            }
            fn apply(&self, args: &s3_core::ArgMap) -> Self {
                Self { #(#applies),* }
            }
            fn to_args(&self) -> s3_core::ArgMap {
                let mut m = s3_core::ArgMap::new();
                #(#seeds)*
                m
            }
        }
    }.into()
}

#[proc_macro_derive(ControlEnum)]
pub fn derive_control_enum(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let Data::Enum(de) = &ast.data else {
        return syn::Error::new_spanned(&ast.ident, "ControlEnum only applies to enums")
            .to_compile_error().into();
    };
    if let Some(v) = de.variants.iter().find(|v| !matches!(v.fields, Fields::Unit)) {
        return syn::Error::new_spanned(v, "ControlEnum needs unit variants only")
            .to_compile_error().into();
    }

    let idents: Vec<_> = de.variants.iter().map(|v| &v.ident).collect();
    let names:  Vec<String> = idents.iter().map(|i| i.to_string()).collect();

    quote! {
        impl s3_core::ControlEnum for #name {
            const VARIANTS: &'static [&'static str] = &[ #(#names),* ];
            fn from_variant(s: &str) -> Option<Self> {
                match s { #(#names => Some(Self::#idents),)* _ => None }
            }
            fn variant_name(&self) -> &'static str {
                match self { #(Self::#idents => #names,)* }
            }
        }
        impl s3_core::FromArg for #name {
            fn from_arg(v: &s3_core::ArgValue) -> Option<Self> {
                match v {
                    s3_core::ArgValue::Variant(s) | s3_core::ArgValue::Text(s) =>
                        <Self as s3_core::ControlEnum>::from_variant(s),
                    _ => None,
                }
            }
        }
        impl s3_core::ToArg for #name {
            fn to_arg(&self) -> s3_core::ArgValue {
                s3_core::ArgValue::Variant(
                    <Self as s3_core::ControlEnum>::variant_name(self).to_string())
            }
        }
    }.into()
}
