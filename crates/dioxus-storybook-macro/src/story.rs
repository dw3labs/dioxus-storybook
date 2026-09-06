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

use crate::{core_path, dioxus_path};

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
    props: Option<Type>,
    tags: Vec<LitStr>,
}

impl Parse for MetaInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut title: Option<LitStr> = None;
        let mut component: Option<Ident> = None;
        let mut props: Option<Type> = None;
        let mut tags: Vec<LitStr> = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "title" => title = Some(input.parse()?),
                "component" => component = Some(input.parse()?),
                "props" => props = Some(input.parse()?),
                "tags" => {
                    let content;
                    syn::bracketed!(content in input);
                    let list =
                        content.parse_terminated(<LitStr as Parse>::parse, Token![,])?;
                    tags = list.into_iter().collect();
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown story_meta key `{other}`; expected one of: title, component, props, tags"
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
            props,
            tags,
        })
    }
}

pub fn story_meta(input: MetaInput) -> TokenStream {
    let core = core_path();
    let dioxus = dioxus_path();
    let MetaInput {
        title,
        component,
        props,
        tags,
    } = input;

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
                .with_tags(&[ #(#tags),* ]);

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
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown #[story] argument `{other}`; expected `name = \"...\"`"),
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
        let body = quote! {
            let __props = <#return_ty as #core::Controllable>::apply(&#call, args);
            #bridge(__props)
        };
        let extras = quote! {
            .with_arg_types(<#return_ty as #core::Controllable>::arg_types)
            .with_base_args(|| <#return_ty as #core::Controllable>::to_args(&#seed_call))
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
            .with_tags(#meta.tags());
    }
}
