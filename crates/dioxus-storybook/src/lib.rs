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
//! M2 — controls and actions. Browse and render stories, edit every prop live
//! from a panel generated out of the props type, watch the component call its
//! own event handlers, and share the result as a URL.
//!
//! # Design notes worth knowing
//!
//! - **Stories are `fn` pointers.** `rsx!` and `EventHandler::new` need an
//!   active Dioxus scope, so story bodies run inside the preview component.
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
pub use dioxus_storybook_ui::{MANAGER_CSS, Storybook};

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
        ControlEnum as ControlEnumTrait, Controllable, FromArg, Meta, ParamValue, Parameters,
        Registry, StoryDef, ToArg, use_args,
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
