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
/// design system's stylesheet, font links or theme provider belong.
static PROJECT: Project = Project::new()
    .with_decorators(&[decorate])
    .with_globals(GLOBALS)
    // `fullscreen` because the decorator below paints the background: canvas
    // padding is surface the decorator cannot reach. A story that wants the
    // default framing can say so itself with its own `parameters`.
    .with_parameters(Parameters::from_static(&[("layout", ParamValue::Str("fullscreen"))]));

/// The toolbar. A global is not a prop: no story declares one, and it survives
/// moving between stories — which is the point. Flip the theme, then walk the
/// sidebar looking for the component that forgot about it.
static GLOBALS: &[GlobalType] = &[
    GlobalType::select("theme", &["light", "dark", "sepia"])
        .with_title("Theme")
        .with_description("Which palette every story renders in"),
    GlobalType::toggle("grid", false)
        .with_title("Grid")
        .with_description("Show an alignment grid behind the story"),
];

/// Reads the toolbar and paints the canvas accordingly.
fn decorate(ctx: &StoryContext, story: Element) -> Element {
    let theme = ctx
        .globals()
        .get("theme")
        .map(ArgValue::as_text)
        .unwrap_or_default();
    let (bg, fg) = match theme.as_str() {
        "dark" => ("#17191C", "#F2F2EF"),
        "sepia" => ("#F4ECD8", "#3B2F1E"),
        _ => ("#FFFFFF", "#17191C"),
    };
    let grid = ctx
        .globals()
        .get("grid")
        .and_then(ArgValue::as_bool)
        .unwrap_or(false);
    let overlay = if grid {
        "background-image:linear-gradient(to right,rgba(127,127,127,.18) 1px,transparent 1px),         linear-gradient(to bottom,rgba(127,127,127,.18) 1px,transparent 1px);         background-size:8px 8px;"
    } else {
        ""
    };
    let style = format!(
        "font-family:ui-sans-serif,system-ui,sans-serif;display:grid;place-items:center;         min-height:100vh;width:100%;background:{bg};color:{fg};{overlay}"
    );
    rsx! {
        div { style: "{style}", {story} }
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
