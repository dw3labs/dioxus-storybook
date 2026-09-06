//! A native [Storybook](https://storybook.js.org/) for [Dioxus](https://dioxuslabs.com):
//! a component workbench where each component's states are declared as
//! *stories*, rendered in isolation, and browsed in a sidebar.
//!
//! # Quick start
//!
//! `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! dioxus = { version = "0.7", features = ["web"] }
//! dioxus-storybook = "0.1"
//!
//! [build-dependencies]
//! dioxus-storybook-build = "0.1"
//! ```
//!
//! `build.rs` — discovers stories so they survive wasm linking:
//!
//! ```ignore
//! fn main() {
//!     dioxus_storybook_build::index("src/stories").unwrap();
//! }
//! ```
//!
//! `src/stories/button.rs` — one file per component:
//!
//! ```ignore
//! use dioxus::prelude::*;
//! use dioxus_storybook::prelude::*;
//! use crate::button::{Button, ButtonProps, ButtonVariant};
//!
//! story_meta! {
//!     title: "Forms/Button",
//!     component: Button,
//! }
//!
//! fn base() -> ButtonProps {
//!     ButtonProps { label: "Click me".into(), variant: ButtonVariant::Primary }
//! }
//!
//! #[story]
//! fn primary() -> ButtonProps { base() }
//!
//! #[story]
//! fn danger() -> ButtonProps {
//!     ButtonProps { variant: ButtonVariant::Danger, ..base() }
//! }
//! ```
//!
//! `src/main.rs`:
//!
//! ```ignore
//! use dioxus::prelude::*;
//! use dioxus_storybook::prelude::*;
//!
//! mod button;
//! mod stories {
//!     include!(concat!(env!("OUT_DIR"), "/dioxus_storybook_registry.rs"));
//! }
//!
//! fn main() {
//!     dioxus::launch(|| rsx! { Storybook { registry: stories::registry() } });
//! }
//! ```
//!
//! Then `dx serve --platform web`.
//!
//! # Status
//!
//! M3 in progress — isolation. Browse and render stories, edit every prop live
//! from a panel generated out of the props type, watch the component call its
//! own event handlers, share the result as a URL, and render every story in a
//! document of its own.
//!
//! # The one thing to know about M3
//!
//! **Stories render in an iframe.** That document has none of your app's CSS or
//! assets in it — which is the point, because a workbench that styles the thing
//! under test is lying to you. Components carrying their own styles are
//! unaffected. Anything that needs a global stylesheet, a font, or a theme
//! provider gets it from a *decorator*:
//!
//! ```ignore
//! static PROJECT: Project = Project::new().with_decorators(&[with_app_css]);
//!
//! fn with_app_css(_ctx: &StoryContext, story: Element) -> Element {
//!     rsx! {
//!         document::Link { rel: "stylesheet", href: asset!("/assets/app.css") }
//!         {story}
//!     }
//! }
//!
//! fn main() {
//!     dioxus::launch(|| rsx! {
//!         Storybook { registry: stories::registry(), project: PROJECT }
//!     });
//! }
//! ```
//!
//! Decorators come at three levels — project, `story_meta!`, `#[story]` — and
//! nest in that order, outermost first. Parameters come at the same three and
//! merge the other way: the innermost level that sets a key wins.
//!
//! # Globals
//!
//! A global is a value the *toolbar* selects and every story sees — a theme, a
//! locale, a text direction. It is not a prop: no story declares one, and it
//! survives moving between stories, which is the point. Flip the theme, then
//! walk the sidebar looking for the component that forgot about it.
//!
//! ```ignore
//! static GLOBALS: &[GlobalType] = &[
//!     GlobalType::select("theme", &["light", "dark"]).with_title("Theme"),
//! ];
//! static PROJECT: Project = Project::new()
//!     .with_globals(GLOBALS)
//!     .with_decorators(&[themed]);
//!
//! fn themed(ctx: &StoryContext, story: Element) -> Element {
//!     let theme = ctx.globals().get("theme").map(ArgValue::as_text).unwrap_or_default();
//!     rsx! { div { class: "{theme}", {story} } }
//! }
//! ```
//!
//! Globals live in the URL too (`&globals=theme:dark`), so a link carries the
//! toolbar with it.
//!
//! # `parameters.layout`
//!
//! One parameter is read by the canvas itself: `layout` is `"centered"` (the
//! default), `"padded"`, or `"fullscreen"`. A decorator that paints a background
//! wants `"fullscreen"` — canvas padding is surface a decorator cannot reach.
//!
//! # Design notes worth knowing
//!
//! - **Stories are `fn` pointers.** `rsx!` and `EventHandler::new` need an
//!   active Dioxus scope, so story bodies run inside the preview component.
//! - **Two documents, one bundle.** The manager points an iframe at its own URL
//!   with `?viewMode=preview`; the copy that loads there renders the canvas
//!   alone. One build, real isolation, and the shell keeps the story registry —
//!   so nothing on the wire needs a serialisable mirror of a `&'static` type.
//! - **Args are overlaid, not deserialised.** Props hold `EventHandler` and
//!   `Element`; serde cannot touch them, so `#[derive(Controls)]` emits a
//!   compiler-checked *applier* instead.
//! - **Actions are wired by substitution, not reflection.** The same derive
//!   emits a pass that replaces each `EventHandler` prop with one that reports
//!   its calls first. The payload prints when it implements `Debug` and
//!   degrades to a placeholder when it does not, so nothing is required of
//!   your types.
//! - **The controls panel never touches the preview.** It edits the manager's
//!   arg set, which goes out on the channel; a story writes back the same way,
//!   with [`use_args`](prelude::use_args).
//! - **The registry is generated at build time,** because `linkme` does not
//!   compile for wasm and `inventory` silently drops stories at
//!   `codegen-units > 1`.
//! - **The URL is load-bearing.** `dx serve` cannot hot-patch on wasm, so a
//!   source edit reloads the page and destroys every signal; the query string
//!   is what brings you back to where you were.

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub use dioxus_storybook_core::*;
pub use dioxus_storybook_macro::{ControlEnum, Controls, story, story_meta};
pub use dioxus_storybook_ui::{
    MANAGER_CSS, PREVIEW_CSS, Storybook, StorybookManager, StorybookPreview,
};

/// Everything you need in a stories file.
///
/// This re-exports [`dioxus::prelude`] as well, so one import is enough. It has
/// to: the `story_meta!` bridge expands `rsx!` inside your stories file, and
/// `rsx!` names `dioxus_core` and `dioxus_signals` as crates that must already
/// be in scope. Importing `dioxus::prelude::*` alongside this is harmless —
/// both globs resolve to the same items.
pub mod prelude {
    pub use dioxus::prelude::*;

    pub use dioxus_storybook_core::{
        ActionSink, ArgMap, ArgType, ArgValue, ArgsHandle, Control,
        ControlEnum as ControlEnumTrait, Controllable, Decorator, FromArg, GlobalType, Meta,
        ParamValue, Parameters, Project, Registry, ResolvedParameters, StoryContext, StoryDef,
        ToArg, use_args,
    };
    pub use dioxus_storybook_macro::{ControlEnum, Controls, story, story_meta};
    pub use dioxus_storybook_ui::Storybook;
}

/// Implementation detail: the paths the proc macros expand to.
///
/// Not part of the public API and exempt from semver.
#[doc(hidden)]
pub mod __private {
    pub use dioxus;
    pub use dioxus_storybook_core as core;
}
