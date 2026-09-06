//! A storybook for two small components.
//!
//! Run with:
//!
//! ```text
//! cd examples/button-gallery && dx serve --platform web
//! ```

use dioxus::prelude::*;
use dioxus_storybook::prelude::*;

mod badge;
mod button;
mod field;

/// The generated registry. `build.rs` both discovers the story files and
/// declares them as modules, so this is the only wiring the app needs.
mod stories {
    include!(concat!(env!("OUT_DIR"), "/dioxus_storybook_registry.rs"));
}

fn main() {
    dioxus::launch(|| rsx! { Storybook { registry: stories::registry() } });
}
