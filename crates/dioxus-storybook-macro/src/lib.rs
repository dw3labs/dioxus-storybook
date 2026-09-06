//! Proc macros for [`dioxus-storybook`].
//!
//! Everything here is re-exported by the `dioxus-storybook` prelude; you should
//! not need to depend on this crate directly. The generated code refers to
//! `::dioxus_storybook`, so the facade crate must be in scope.
//!
//! [`dioxus-storybook`]: https://crates.io/crates/dioxus-storybook

#![deny(missing_docs)]

mod props;
mod story;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, ItemFn, parse_macro_input};

/// Path to the core crate, as re-exported by the facade.
fn core_path() -> TokenStream2 {
    quote!(::dioxus_storybook::__private::core)
}

/// Path to Dioxus, as re-exported by the facade, so users need not name it.
fn dioxus_path() -> TokenStream2 {
    quote!(::dioxus_storybook::__private::dioxus)
}

/// Generate a props table, an arg applier and an arg seeder for a props struct.
///
/// The derive reads field names, types and `///` doc comments, so the controls
/// panel and the docs page come from the type itself with no configuration.
///
/// # Field attributes
///
/// - `#[control(skip)]` — no widget; the field is carried through untouched.
/// - `#[control(text)]`, `#[control(number)]`, `#[control(color)]`
/// - `#[control(select)]`, `#[control(radio)]` — for `#[derive(ControlEnum)]` types
/// - `#[control(range(min = 0.0, max = 4.0, step = 0.25))]`
///
/// Without an attribute the widget is inferred from the type. `EventHandler`
/// fields become actions, `Element` fields get no widget, and `Option<T>` is
/// marked optional.
///
/// # Example
///
/// ```ignore
/// #[derive(Props, Clone, PartialEq, Controls)]
/// pub struct ButtonProps {
///     /// Text shown inside the button.
///     pub label: String,
///     /// Visual emphasis.
///     pub variant: ButtonVariant,
///     #[control(range(min = 0.75, max = 2.5, step = 0.05))]
///     pub scale: f32,
///     pub onclick: EventHandler<MouseEvent>,
/// }
/// ```
#[proc_macro_derive(Controls, attributes(control))]
pub fn derive_controls(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    props::derive_controls(ast).into()
}

/// Make a unit-only enum selectable from a control.
///
/// Supplies the variant names to `Select`/`Radio` controls and implements the
/// conversions that let a variant survive a round trip through a URL.
///
/// ```ignore
/// #[derive(Clone, Copy, PartialEq, ControlEnum)]
/// pub enum ButtonVariant { Primary, Secondary, Ghost, Danger }
/// ```
#[proc_macro_derive(ControlEnum)]
pub fn derive_control_enum(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    props::derive_control_enum(ast).into()
}

/// Declare the component-level metadata shared by every story in a module.
///
/// Place it once, at the top of a stories file, before any `#[story]`.
///
/// ```ignore
/// story_meta! {
///     title: "Forms/Button",   // the sidebar path, split on `/`
///     component: Button,       // the Dioxus component these stories render
///     tags: ["autodocs"],      // optional
/// }
/// ```
///
/// # Keys
///
/// - `title` — required. Slash-separated; the sidebar tree is built from it.
/// - `component` — required. The component every props-form story renders.
/// - `props` — optional. The component's props type. Defaults to
///   `{component}Props`, the Dioxus `#[component]` convention, so you only need
///   it when your props struct is named differently:
///   `props: MyButtonProperties`.
/// - `tags` — optional. Inherited by every story in the module.
#[proc_macro]
pub fn story_meta(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as story::MetaInput);
    story::story_meta(parsed).into()
}

/// Turn a function into a story.
///
/// Two forms are accepted:
///
/// - **Props form** — returns the component's props type. The dynamic args are
///   overlaid onto what the function returns, and the component is rendered for
///   you. This is the form that gets controls and a props table.
/// - **Element form** — returns `Element`, optionally taking `&ArgMap`. Full
///   control over the markup, no auto-generated controls.
///
/// The display name defaults to the function name in title case
/// (`with_icon` → `With Icon`) and can be overridden with `name = "..."`.
///
/// ```ignore
/// #[story]
/// fn primary() -> ButtonProps { ButtonProps { ..base() } }
///
/// #[story(name = "With Icon")]
/// fn with_icon(args: &ArgMap) -> Element { rsx! { /* ... */ } }
/// ```
///
/// # How it reaches the registry
///
/// The macro emits a `StoryDef` static named after the function in
/// SCREAMING_SNAKE case. `dioxus-storybook-build` scans your story files for
/// `#[story]` at build time and emits explicit references to those statics —
/// which is why stories survive `wasm32` linking, where `linkme` does not
/// compile and `inventory` silently drops them.
#[proc_macro_attribute]
pub fn story(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as story::StoryArgs);
    let func = parse_macro_input!(item as ItemFn);
    story::story(args, func).into()
}
