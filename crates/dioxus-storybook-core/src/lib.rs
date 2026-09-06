//! Core types for [`dioxus-storybook`], a native Storybook for Dioxus.
//!
//! This crate is the part that has no opinion about rendering: the story
//! description ([`StoryDef`], [`Meta`]), the dynamic-argument vocabulary
//! ([`ArgType`], [`ArgValue`], [`ArgMap`]), the sidebar index ([`Registry`]),
//! URL state ([`url`]), and the manager ↔ preview bus ([`Channel`]).
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
pub mod registry;
pub mod story;
pub mod url;

pub use actions::{ActionSink, ArgsHandle, use_args};
pub use arg_type::{ArgType, Control};
pub use args::{ArgMap, ArgValue, ControlEnum, Controllable, FromArg, ToArg};
pub use channel::{Channel, Event, InProcessChannel, Listener, Subscription};
pub use registry::{
    Group, Registry, Row, RowKind, StoryRef, TreeNode, flatten, fuzzy_score, group_paths,
};
pub use story::{Meta, ParamValue, Parameters, StoryDef, kebab};
pub use url::UrlState;
