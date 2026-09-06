//! `story_meta!` and `#[story]` — the authoring surface.
//!
//! ```ignore
//! story_meta! {
//!     title: "Forms/Button",
//!     component: Button,
//!     tags: ["autodocs"],
//! }
//!
//! #[story]
//! fn primary() -> ButtonProps { ButtonProps { ..base() } }
//!
//! #[story(name = "With Icon")]
//! fn with_icon(args: &ArgMap) -> Element { rsx! { /* ... */ } }
//! ```
//!
//! `story_meta!` emits two module-local items: a `__DXSB_META` const, and a
//! `__dxsb_render_component` function that renders *this module's* component
//! from its props. `#[story]` then emits a `StoryDef` static whose name is the
//! SCREAMING_SNAKE form of the function name — which is exactly what
//! `dioxus-storybook-build` scans for when it generates the registry. The two
//! must agree; see `dioxus_storybook_build::static_ident_for`.
//!
//! # Why the bridge is a function and not a `macro_rules!`
//!
//! `#[story]` knows the props type (the story function's return type) but not
//! the component; `story_meta!` knows the component but not the props type. The
//! obvious way to let them meet is for `story_meta!` to emit a `macro_rules!`
//! that `#[story]` invokes. **That version compiles under rustc and breaks
//! rust-analyzer**: a value created in the attribute expansion and passed as an
//! argument into a proc-macro-generated `macro_rules!` fails to resolve, so
//! every props-form story shows a false `E0425: no such value in this scope`.
//! Compare rust-analyzer#10644, open since 2021.
//!
//! Referencing a plain module *item* across the same expansion boundary is
//! fine, so the bridge is a concrete function instead. That costs `story_meta!`
//! knowing the props type, which it infers as `{Component}Props` — the Dioxus
//! `#[component]` convention — and which `props: ...` overrides.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, LitStr, ReturnType, Token, Type};

use crate::props::docs_of;
use crate::{core_path, dioxus_path};

/// The story's body, as the author wrote it, for the docs page's source block.
///
/// Taken from the *source text* of the block's span rather than from its token
/// stream, so the snippet keeps the author's line breaks, indentation and
/// comments. `quote!(#block).to_string()` would give
/// `{ ButtonProps { variant : ... , .. base () } }`, which is a rendering of
/// the tokens and not the code anyone wrote.
///
/// `source_text` is `None` when the span has no file behind it — a story that
/// was itself produced by another macro. That falls back to the token stream,
/// which is ugly but true.
fn body_snippet(block: &syn::Block) -> String {
    let raw = block
        .brace_token
        .span
        .join()
        .source_text()
        .unwrap_or_else(|| quote!(#block).to_string());
    dedent(strip_braces(&raw))
}

/// Drop the outer `{` / `}` of a block's source. The braces belong to the
/// function, not to the example.
fn strip_braces(raw: &str) -> &str {
    raw.trim()
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(raw)
}

/// Remove the common leading whitespace, so a body nested two levels deep in a
/// file does not arrive on the docs page indented by eight columns.
fn dedent(text: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .skip_while(|l| l.trim().is_empty())
        .collect();
    let end = lines
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map_or(0, |i| i + 1);
    let lines = &lines[..end];
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|l| if l.len() >= indent { &l[indent..] } else { l.trim_start() })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One `key: value` pair inside a `parameters { .. }` block.
///
/// Keys may be written bare (`layout: "centered"`) or quoted, because a
/// parameter name is a string at run time and addons are free to use ones that
/// are not Rust identifiers.
struct Param {
    key: String,
    value: TokenStream,
}

/// Parse `{ layout: "centered", padding: 16.0, docs: false }`.
fn parse_parameters(input: ParseStream) -> syn::Result<Vec<Param>> {
    let content;
    syn::braced!(content in input);
    let core = core_path();
    let mut out = Vec::new();
    while !content.is_empty() {
        let key = if content.peek(LitStr) {
            content.parse::<LitStr>()?.value()
        } else {
            content.parse::<Ident>()?.to_string()
        };
        content.parse::<Token![:]>()?;
        let lit: syn::Lit = content.parse()?;
        let value = match &lit {
            syn::Lit::Str(s) => quote!(#core::ParamValue::Str(#s)),
            syn::Lit::Bool(b) => quote!(#core::ParamValue::Bool(#b)),
            syn::Lit::Float(f) => quote!(#core::ParamValue::Num(#f)),
            syn::Lit::Int(i) => {
                // Written as an integer, stored as the `f64` every other number
                // parameter is, so `padding: 16` and `padding: 16.0` mean the
                // same thing to an addon.
                let as_f64 = i.base10_parse::<i64>()? as f64;
                quote!(#core::ParamValue::Num(#as_f64))
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "a parameter value must be a string, number or bool",
                ));
            }
        };
        out.push(Param { key, value });
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(out)
}

/// Parse `[with_theme, padded]` — paths to `Decorator` fn items or closures.
fn parse_decorators(input: ParseStream) -> syn::Result<Vec<syn::Expr>> {
    let content;
    syn::bracketed!(content in input);
    let list = content.parse_terminated(<syn::Expr as Parse>::parse, Token![,])?;
    Ok(list.into_iter().collect())
}

/// `Parameters::from_static(&[..])`, or nothing when there are none.
fn parameters_expr(params: &[Param]) -> TokenStream {
    let core = core_path();
    if params.is_empty() {
        return quote!(#core::Parameters::new());
    }
    let entries = params.iter().map(|Param { key, value }| quote!((#key, #value)));
    quote!(#core::Parameters::from_static(&[ #(#entries),* ]))
}

/// `&[..]` of decorators, typed so a bare closure coerces to a `fn` pointer.
fn decorators_expr(decorators: &[syn::Expr]) -> TokenStream {
    let core = core_path();
    quote!({
        const __DXSB_DECORATORS: &[#core::Decorator] = &[ #(#decorators),* ];
        __DXSB_DECORATORS
    })
}

/// The identifier `story_meta!` defines and `#[story]` reads.
fn meta_ident() -> Ident {
    Ident::new("__DXSB_META", Span::call_site())
}

/// The generated function that turns a props value into an `Element`.
fn bridge_ident() -> Ident {
    Ident::new("__dxsb_render_component", Span::call_site())
}

// ---------------------------------------------------------------- story_meta

pub struct MetaInput {
    title: LitStr,
    component: Ident,
    description: Option<LitStr>,
    props: Option<Type>,
    tags: Vec<LitStr>,
    parameters: Vec<Param>,
    decorators: Vec<syn::Expr>,
}

impl Parse for MetaInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut title: Option<LitStr> = None;
        let mut component: Option<Ident> = None;
        let mut description: Option<LitStr> = None;
        let mut props: Option<Type> = None;
        let mut tags: Vec<LitStr> = Vec::new();
        let mut parameters: Vec<Param> = Vec::new();
        let mut decorators: Vec<syn::Expr> = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "title" => title = Some(input.parse()?),
                "component" => component = Some(input.parse()?),
                "description" => description = Some(input.parse()?),
                "props" => props = Some(input.parse()?),
                "tags" => {
                    let content;
                    syn::bracketed!(content in input);
                    let list =
                        content.parse_terminated(<LitStr as Parse>::parse, Token![,])?;
                    tags = list.into_iter().collect();
                }
                "parameters" => parameters = parse_parameters(input)?,
                "decorators" => decorators = parse_decorators(input)?,
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown story_meta key `{other}`; expected one of: \
                             title, component, description, props, tags, parameters, \
                             decorators"
                        ),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let title = title.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "story_meta! needs a `title:` — it is the sidebar path, e.g. \"Forms/Button\"",
            )
        })?;
        let component = component.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "story_meta! needs a `component:` — the Dioxus component these stories render",
            )
        })?;

        Ok(Self {
            title,
            component,
            description,
            props,
            tags,
            parameters,
            decorators,
        })
    }
}

pub fn story_meta(input: MetaInput) -> TokenStream {
    let core = core_path();
    let dioxus = dioxus_path();
    let MetaInput {
        title,
        component,
        description,
        props,
        tags,
        parameters,
        decorators,
    } = input;
    let description = description.map_or_else(String::new, |d| d.value());
    let parameters = parameters_expr(&parameters);
    let decorators = decorators_expr(&decorators);

    let component_name = component.to_string();
    let meta = meta_ident();
    let bridge = bridge_ident();

    // Default to the Dioxus `#[component]` convention, `Button` -> `ButtonProps`.
    let props_ty: Type = props.unwrap_or_else(|| {
        let ident = Ident::new(&format!("{component_name}Props"), component.span());
        syn::parse_quote!(#ident)
    });

    quote! {
        /// Component-level story metadata, generated by `story_meta!`.
        #[doc(hidden)]
        pub const #meta: #core::Meta =
            #core::Meta::new(#title, #component_name)
                .with_description(#description)
                .with_tags(&[ #(#tags),* ])
                .with_parameters(#parameters)
                .with_decorators(#decorators);

        /// Renders this module's component from its props. Generated by
        /// `story_meta!` so that `#[story]` never has to name the component.
        #[doc(hidden)]
        #[allow(dead_code)]
        pub fn #bridge(props: #props_ty) -> #dioxus::prelude::Element {
            #dioxus::prelude::rsx! { #component { ..props } }
        }
    }
}

// -------------------------------------------------------------------- #[story]

#[derive(Default)]
pub struct StoryArgs {
    name: Option<LitStr>,
    parameters: Vec<Param>,
    decorators: Vec<syn::Expr>,
}

impl Parse for StoryArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = StoryArgs::default();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            match key.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    args.name = Some(input.parse()?);
                }
                // `=` before the block reads oddly in an attribute, so both
                // `parameters { .. }` and `parameters = { .. }` are accepted.
                "parameters" => {
                    let _ = input.parse::<Token![=]>();
                    args.parameters = parse_parameters(input)?;
                }
                "decorators" => {
                    let _ = input.parse::<Token![=]>();
                    args.decorators = parse_decorators(input)?;
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown #[story] argument `{other}`; expected one of: \
                             name, parameters, decorators"
                        ),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(args)
    }
}

/// `with_icon` -> `"With Icon"`.
fn display_name(ident: &Ident) -> String {
    ident
        .to_string()
        .split('_')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `with_icon` -> `WITH_ICON`. Must match `dioxus_storybook_build`.
fn static_ident(ident: &Ident) -> Ident {
    Ident::new(&ident.to_string().to_uppercase(), ident.span())
}

/// Is the function's return type literally `Element`?
fn returns_element(ret: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = ret else {
        return false;
    };
    let Type::Path(p) = &**ty else { return false };
    p.path
        .segments
        .last()
        .is_some_and(|s| s.ident == "Element")
}

pub fn story(args: StoryArgs, func: ItemFn) -> TokenStream {
    let core = core_path();
    let fn_ident = func.sig.ident.clone();
    let meta = meta_ident();
    let bridge = bridge_ident();

    let display = args
        .name
        .map(|l| l.value())
        .unwrap_or_else(|| display_name(&fn_ident));
    // The story's own `///`, and its body as written. Both are read straight
    // off the item the attribute was placed on, which is why autodocs needs no
    // docgen pass and no sidecar file.
    let story_docs = docs_of(&func.attrs);
    let source = body_snippet(&func.block);
    let parameters = parameters_expr(&args.parameters);
    let decorators = decorators_expr(&args.decorators);
    let static_name = static_ident(&fn_ident);

    let ReturnType::Type(_, return_ty) = &func.sig.output else {
        return syn::Error::new_spanned(
            &func.sig,
            "a #[story] must return either its component's Props type or `Element`",
        )
        .to_compile_error();
    };
    let return_ty = (**return_ty).clone();

    let takes_args = !func.sig.inputs.is_empty();

    // A story function is called for two different reasons: to render (with the
    // caller's args overlaid) and to seed the controls panel and URL (with no
    // overlay). Both need the same call shape.
    let call = if takes_args {
        quote!(#fn_ident(args))
    } else {
        quote!(#fn_ident())
    };
    let seed_call = if takes_args {
        quote!(#fn_ident(&#core::ArgMap::new()))
    } else {
        quote!(#fn_ident())
    };

    let (render_body, extras) = if returns_element(&func.sig.output) {
        // Escape hatch: the story builds its own rsx, and opts out of controls
        // because there is no props type to introspect.
        let body = if takes_args {
            quote!(#call)
        } else {
            quote!({ let _ = args; #call })
        };
        (body, quote!())
    } else {
        // Props form: overlay the dynamic args onto the story's typed props,
        // then hand the result to the component through the bridge.
        // Overlay first, instrument second. `apply` carries handler fields
        // through untouched, so this order leaves exactly one wrapper on each
        // handler; the reverse order would work too but reads backwards.
        let body = quote! {
            let __props = <#return_ty as #core::Controllable>::apply(&#call, args);
            let __props = <#return_ty as #core::Controllable>::wire_actions(
                &__props,
                &#core::ActionSink::ambient(),
            );
            #bridge(__props)
        };
        let extras = quote! {
            .with_arg_types(<#return_ty as #core::Controllable>::arg_types)
            .with_base_args(|| <#return_ty as #core::Controllable>::to_args(&#seed_call))
            // The props type's own doc comment, as the docs page's fallback
            // description. An associated *const*, so it is readable here in the
            // `const` initialiser this whole static is.
            .with_component_docs(<#return_ty as #core::Controllable>::DOCS)
        };
        (body, extras)
    };

    quote! {
        #func

        #[doc = concat!("Story `", #display, "`, generated by `#[story]`.")]
        // Always `pub`: the generated registry lives in an ancestor module and
        // has to be able to name this static. The story file itself is only
        // reachable through that generated module, so nothing leaks further.
        #[allow(non_upper_case_globals)]
        pub static #static_name: #core::StoryDef =
            #core::StoryDef::new(#meta.title(), #display, |args: &#core::ArgMap| {
                #render_body
            })
            #extras
            .with_docs(#story_docs)
            .with_source(#source)
            .with_tags(#meta.tags())
            .with_parameters(#parameters)
            .with_decorators(#decorators)
            // Carries the component level whole, so the story can reach the
            // parameters and decorators declared once in `story_meta!`.
            .with_meta(#meta);
    }
}
