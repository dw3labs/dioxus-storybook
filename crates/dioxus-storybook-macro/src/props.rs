//! `#[derive(Controls)]` and `#[derive(ControlEnum)]`.
//!
//! There is no `react-docgen` for Rust, and that turns out to be an advantage
//! rather than a gap: a derive macro sees field names, types and doc comments
//! statically, and the applier it emits is checked by the compiler. Validated
//! by the M0 S3 spike against real Dioxus types (`Props`, `Element`,
//! `EventHandler<MouseEvent>`).

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Data, DeriveInput, Fields, Type};

use crate::core_path;

/// The **summary paragraph** of the `///` doc comments on a field or item.
///
/// Lines up to the first blank one, joined. That is rustdoc's own rule for the
/// short description it puts in an item listing, and it is the right rule here
/// for the same reason: a docs page shows one blurb per story next to the
/// example, and a doc comment whose second paragraph explains an implementation
/// detail should not push the next example off the screen. The full text is
/// still what `cargo doc` renders — this only decides what the storybook quotes.
pub(crate) fn docs_of(attrs: &[syn::Attribute]) -> String {
    let mut out: Vec<String> = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(nv) = &attr.meta
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
        {
            let line = s.value().trim().to_string();
            // A blank line ends the summary — but only once something has been
            // collected, so a leading `///` on its own is skipped rather than
            // ending the paragraph before it starts.
            if line.is_empty() {
                if out.is_empty() {
                    continue;
                }
                break;
            }
            out.push(line);
        }
    }
    out.join(" ").trim().to_string()
}

/// The outermost path segment of a type, e.g. `Option<T>` -> `"Option"`.
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
    if seg.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(a) = &seg.arguments else {
        return None;
    };
    a.args.iter().find_map(|g| match g {
        syn::GenericArgument::Type(t) => Some(t),
        _ => None,
    })
}

/// An explicit `#[control(...)]` override.
enum Override {
    Skip,
    Text,
    Number,
    Color,
    Select,
    Radio,
    Range(f64, f64, f64),
}

fn control_attr(attrs: &[syn::Attribute]) -> Option<Override> {
    let attr = attrs.iter().find(|a| a.path().is_ident("control"))?;
    let mut found = None;
    let _ = attr.parse_nested_meta(|m| {
        let id = m
            .path
            .get_ident()
            .map(ToString::to_string)
            .unwrap_or_default();
        match id.as_str() {
            "skip" => found = Some(Override::Skip),
            "text" => found = Some(Override::Text),
            "number" => found = Some(Override::Number),
            "color" => found = Some(Override::Color),
            "select" => found = Some(Override::Select),
            "radio" => found = Some(Override::Radio),
            "range" => {
                let (mut lo, mut hi, mut step) = (0.0f64, 1.0f64, 0.01f64);
                let _ = m.parse_nested_meta(|r| {
                    let key = r
                        .path
                        .get_ident()
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    let lit: syn::Lit = r.value()?.parse()?;
                    let n = match lit {
                        syn::Lit::Float(f) => f.base10_parse::<f64>().unwrap_or(0.0),
                        syn::Lit::Int(i) => i.base10_parse::<f64>().unwrap_or(0.0),
                        _ => 0.0,
                    };
                    match key.as_str() {
                        "min" => lo = n,
                        "max" => hi = n,
                        "step" => step = n,
                        _ => {}
                    }
                    Ok(())
                });
                found = Some(Override::Range(lo, hi, step));
            }
            _ => {}
        }
        Ok(())
    });
    found
}

/// Syntactic control inference.
///
/// Deliberately conservative: an unrecognised type falls through to `Select`
/// via [`ControlEnum`], and anything that is not an enum fails to compile with
/// a message pointing at the missing derive rather than at macro internals.
fn infer(ty: &Type) -> TokenStream {
    let core = core_path();
    let base = option_inner(ty).unwrap_or(ty);
    let h = head(base).unwrap_or_default();
    match h.as_str() {
        "String" | "str" => quote!(#core::Control::Text),
        "bool" => quote!(#core::Control::Toggle),
        "f32" | "f64" | "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64"
        | "usize" => quote!(#core::Control::Number { min: None, max: None, step: None }),
        "EventHandler" | "Callback" => quote!(#core::Control::Action),
        "Element" | "VNode" => quote!(#core::Control::None),
        // A list is edited as comma-separated text; `FromArg for Vec<T>` parses
        // that form back. A dedicated list widget can arrive later without
        // changing anything here — `Control` is `#[non_exhaustive]`.
        "Vec" => quote!(#core::Control::Text),
        _ => quote!(#core::Control::Select {
            options: <#base as #core::ControlEnum>::VARIANTS
        }),
    }
}

/// Is this field an event handler, and therefore instrumentable by
/// `wire_actions`?
///
/// Deliberately syntactic, matching [`infer`]: a field is wrapped only when it
/// is spelled `EventHandler<..>` or `Callback<..>`. The wrapper is built with
/// `<FieldType>::new(..)`, so `Callback<A, R>` works as well as
/// `EventHandler<A>` — the closure simply returns whatever the inner handler
/// returns.
fn is_handler(ty: &Type) -> bool {
    matches!(
        head(ty).unwrap_or_default().as_str(),
        "EventHandler" | "Callback"
    )
}

/// Does this field get a live control, and therefore participate in `apply`?
fn is_dynamic(ty: &Type, ov: Option<&Override>) -> bool {
    if matches!(ov, Some(Override::Skip)) {
        return false;
    }
    let base = option_inner(ty).unwrap_or(ty);
    !matches!(
        head(base).unwrap_or_default().as_str(),
        "EventHandler" | "Callback" | "Element" | "VNode"
    )
}

pub fn derive_controls(ast: DeriveInput) -> TokenStream {
    let core = core_path();
    let name = &ast.ident;
    // The props type's own `///`. The docs page falls back to it when
    // `story_meta!` gave no description, which is what makes a component
    // documented without anyone writing storybook-specific prose.
    let type_docs = docs_of(&ast.attrs);

    let Data::Struct(ds) = &ast.data else {
        return syn::Error::new_spanned(&ast.ident, "Controls can only be derived for structs")
            .to_compile_error();
    };
    let Fields::Named(named) = &ds.fields else {
        return syn::Error::new_spanned(
            &ast.ident,
            "Controls needs named fields — tuple and unit structs have no prop names to show",
        )
        .to_compile_error();
    };

    let mut rows = Vec::new();
    let mut applies = Vec::new();
    let mut seeds = Vec::new();
    let mut wires = Vec::new();

    for field in &named.named {
        let ident = field.ident.as_ref().expect("named fields");
        let fname = ident.to_string();
        let ty = &field.ty;
        let ov = control_attr(&field.attrs);
        let docs = docs_of(&field.attrs);
        let ty_str = ty.to_token_stream().to_string().replace(' ', "");
        let required = option_inner(ty).is_none();

        let control = match &ov {
            Some(Override::Skip) => quote!(#core::Control::None),
            Some(Override::Text) => quote!(#core::Control::Text),
            Some(Override::Number) => {
                quote!(#core::Control::Number { min: None, max: None, step: None })
            }
            Some(Override::Color) => quote!(#core::Control::Color),
            Some(Override::Select) => {
                let base = option_inner(ty).unwrap_or(ty);
                quote!(#core::Control::Select { options: <#base as #core::ControlEnum>::VARIANTS })
            }
            Some(Override::Radio) => {
                let base = option_inner(ty).unwrap_or(ty);
                quote!(#core::Control::Radio { options: <#base as #core::ControlEnum>::VARIANTS })
            }
            Some(Override::Range(lo, hi, step)) => {
                quote!(#core::Control::Range { min: #lo, max: #hi, step: #step })
            }
            None => infer(ty),
        };

        rows.push(quote! {
            #core::ArgType {
                name: #fname,
                ty: #ty_str,
                docs: #docs,
                control: #control,
                required: #required,
            }
        });

        if is_dynamic(ty, ov.as_ref()) {
            applies.push(quote! { #ident: args.get_or(#fname, self.#ident.clone()) });
            seeds.push(quote! {
                m.set(#fname, #core::ToArg::to_arg(&self.#ident));
            });
        } else {
            applies.push(quote! { #ident: self.#ident.clone() });
        }

        // Handlers are the one field kind `apply` cannot touch, so they get
        // their own pass. `<#ty>::new` rather than `EventHandler::new` so the
        // wrapper keeps the field's own argument and return types.
        // Autoref specialisation: prints the payload when it is `Debug` and
        // degrades to a placeholder when it is not, without putting a bound on
        // the user's type.
        let log = quote! {
            #[allow(unused_imports)]
            use #core::actions::describe::DescribeFallback as _;
            __dxsb_sink.log(
                #fname,
                #core::actions::describe::Probe(&__dxsb_event).dxsb_describe(),
            );
        };
        let skipped = matches!(ov, Some(Override::Skip));
        let handler_ty = (!skipped)
            .then(|| {
                if is_handler(ty) {
                    Some((ty, false))
                } else {
                    option_inner(ty).filter(|t| is_handler(t)).map(|t| (t, true))
                }
            })
            .flatten();

        match handler_ty {
            // `<#inner>::new` rather than `EventHandler::new` so the wrapper
            // keeps the field's own argument and return types — that is what
            // makes this work for `Callback<A, R>` as well as `EventHandler<A>`.
            Some((inner, false)) => wires.push(quote! {
                #ident: {
                    let __dxsb_inner = self.#ident;
                    let __dxsb_sink = ::core::clone::Clone::clone(sink);
                    <#inner>::new(move |__dxsb_event| {
                        #log
                        __dxsb_inner.call(__dxsb_event)
                    })
                }
            }),
            Some((inner, true)) => wires.push(quote! {
                #ident: self.#ident.map(|__dxsb_inner| {
                    let __dxsb_sink = ::core::clone::Clone::clone(sink);
                    <#inner>::new(move |__dxsb_event| {
                        #log
                        __dxsb_inner.call(__dxsb_event)
                    })
                })
            }),
            None => wires.push(quote! { #ident: self.#ident.clone() }),
        }
    }

    quote! {
        impl #core::Controllable for #name {
            const DOCS: &'static str = #type_docs;

            fn arg_types() -> &'static [#core::ArgType] {
                // A `const`: the whole props table is materialised at compile
                // time, so reading it at runtime costs nothing.
                const ROWS: &[#core::ArgType] = &[ #(#rows),* ];
                ROWS
            }

            fn apply(&self, args: &#core::ArgMap) -> Self {
                Self { #(#applies),* }
            }

            fn to_args(&self) -> #core::ArgMap {
                let mut m = #core::ArgMap::new();
                #(#seeds)*
                m
            }

            fn wire_actions(&self, sink: &#core::ActionSink) -> Self {
                // Cheap when the sink is disabled, but not free: building a
                // handler needs a live scope either way, which is exactly why
                // this is a separate pass invoked from the story body rather
                // than something the registry could precompute.
                let _ = sink;
                Self { #(#wires),* }
            }
        }
    }
}

pub fn derive_control_enum(ast: DeriveInput) -> TokenStream {
    let core = core_path();
    let name = &ast.ident;

    let Data::Enum(de) = &ast.data else {
        return syn::Error::new_spanned(&ast.ident, "ControlEnum can only be derived for enums")
            .to_compile_error();
    };
    if let Some(v) = de.variants.iter().find(|v| !matches!(v.fields, Fields::Unit)) {
        return syn::Error::new_spanned(
            v,
            "ControlEnum needs unit variants only — a select control has no way to fill in fields",
        )
        .to_compile_error();
    }

    let idents: Vec<_> = de.variants.iter().map(|v| &v.ident).collect();
    let names: Vec<String> = idents.iter().map(ToString::to_string).collect();

    quote! {
        impl #core::ControlEnum for #name {
            const VARIANTS: &'static [&'static str] = &[ #(#names),* ];

            fn from_variant(name: &str) -> Option<Self> {
                match name { #(#names => Some(Self::#idents),)* _ => None }
            }

            fn variant_name(&self) -> &'static str {
                match self { #(Self::#idents => #names,)* }
            }
        }

        impl #core::FromArg for #name {
            fn from_arg(value: &#core::ArgValue) -> Option<Self> {
                match value {
                    #core::ArgValue::Variant(s) | #core::ArgValue::Text(s) =>
                        <Self as #core::ControlEnum>::from_variant(s),
                    _ => None,
                }
            }
        }

        impl #core::ToArg for #name {
            fn to_arg(&self) -> #core::ArgValue {
                #core::ArgValue::Variant(
                    <Self as #core::ControlEnum>::variant_name(self).to_string()
                )
            }
        }
    }
}
