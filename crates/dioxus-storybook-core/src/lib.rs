//! Core types for [`dioxus-storybook`], a native Storybook for Dioxus.
//!
//! This crate is the part that has no opinion about rendering: the story
//! description ([`StoryDef`], [`Meta`]), the dynamic-argument vocabulary
//! ([`ArgType`], [`ArgValue`], [`ArgMap`]), the sidebar index ([`Registry`]),
//! URL state ([`url`]), the manager ↔ preview bus ([`Channel`]) and its
//! [`wire`] format, and the three things a story inherits rather than declares:
//! [`Decorator`]s, [`Parameters`] and [`GlobalType`]s.
//!
//! # Three value vocabularies, and why there are exactly three
//!
//! | | changes at run time | scope |
//! |---|---|---|
//! | [`ArgValue`] args | yes | one story |
//! | [`ParamValue`] parameters | no | a level: project, component or story |
//! | globals | yes | the whole book |
//!
//! Globals are not a fourth: they are declared with the same [`Control`] a prop
//! uses and carried in an [`ArgMap`], which is what gives them a widget, a URL
//! encoding and a wire encoding for free. See [`globals`].
//!
//! The [`viewport`] addon is the worked example of staying inside those three:
//! its list of sizes is a declaration on the [`Project`], the size a story opens
//! at is a *parameter*, and the size you picked is a *global*. Nothing about it
//! needed a new kind of value.
//!
//! You normally depend on `dioxus-storybook` and use its prelude instead of
//! reaching in here.
//!
//! # The three load-bearing decisions
//!
//! - **Stories are `fn` pointers, not values.** `rsx!` and `EventHandler::new`
//!   need an active Dioxus scope, so story bodies are invoked from inside the
//!   preview component. See [`story`].
//! - **Args are overlaid, not deserialised.** Props hold `EventHandler` and
//!   `Element`, which serde cannot touch. See [`args`].
//! - **The registry is generated at build time.** `linkme` does not compile for
//!   wasm and `inventory` silently drops stories at `codegen-units > 1`. See
//!   [`registry`].
//!
//! [`dioxus-storybook`]: https://crates.io/crates/dioxus-storybook

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod actions;
pub mod args;
pub mod arg_type;
pub mod channel;
pub mod docs;
pub mod globals;
pub mod registry;
pub mod story;
pub mod url;
pub mod viewport;
pub mod wire;

pub use actions::{ActionSink, ArgsHandle, use_args};
pub use arg_type::{ArgType, Control};
pub use args::{ArgMap, ArgValue, ControlEnum, Controllable, FromArg, ToArg};
pub use channel::{Channel, ChannelHandle, Event, InProcessChannel, Listener, Subscription};
pub use docs::{Autodocs, DocsPage, Entry};
pub use globals::GlobalType;
pub use registry::{
    Group, Registry, Row, RowKind, StoryRef, TreeNode, flatten, fuzzy_score, group_paths,
};
pub use story::{
    Decorator, Meta, ParamValue, Parameters, Project, ResolvedParameters, StoryContext,
    StoryDef, StoryView, kebab,
};
pub use url::UrlState;
pub use viewport::{Viewport, ViewportSelection};
pub use wire::ViewMode;
