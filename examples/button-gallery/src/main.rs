//! A storybook for three small components.
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

/// Everything that applies to every story.
///
/// The decorator here is doing the job that matters most from M3 onwards: the
/// story renders in an **iframe**, in a document that inherits none of this
/// app's CSS. A project decorator runs inside that document, so it is where a
/// design system's stylesheet, font links or theme provider belong. This one
/// gives every story a font and a little breathing room, which the shell around
/// it can no longer do for it.
static PROJECT: Project = Project::new()
    .with_decorators(&[decorate])
    .with_parameters(Parameters::from_static(&[("layout", ParamValue::Str("centered"))]));

fn decorate(_ctx: &StoryContext, story: Element) -> Element {
    rsx! {
        div {
            style: "font-family:ui-sans-serif,system-ui,sans-serif;display:grid;place-items:center",
            {story}
        }
    }
}

fn main() {
    dioxus::launch(|| {
        rsx! {
            Storybook {
                registry: stories::registry(),
                project: PROJECT,
            }
        }
    });
}
