//! The generated documentation page for one component.
//!
//! # This renders in the preview, not in the shell
//!
//! Every example on the page is a real story, rendered by the same
//! [`StoryDef::render_decorated`] the canvas uses — same decorators, same
//! parameters, same globals. That is only possible in the preview document:
//! evaluating a story's props builds `EventHandler`s and needs a live Dioxus
//! scope, and decorators are what put the author's own stylesheet inside the
//! frame. A docs page drawn by the manager would show every component styled by
//! the workbench instead, which is the exact failure the M3 split exists to
//! prevent.
//!
//! The consequence is that the page's own chrome — headings, the props table,
//! the source blocks — is styled by [`PREVIEW_CSS`](crate::PREVIEW_CSS) and not
//! by the shell's stylesheet, and that the shell shows a docs entry by pointing
//! the same iframe at it. No new document, no new transport, no new message:
//! `forms-button--docs` is an id like any other.
//!
//! [`StoryDef::render_decorated`]: dioxus_storybook_core::StoryDef::render_decorated

use dioxus::prelude::*;
use dioxus_storybook_core::{ArgMap, ArgType, Control, DocsPage, Project, StoryDef, StoryView};

use crate::inline::Doc;
use crate::StoryHost;

/// One component's documentation: description, props table, and every story.
#[component]
pub fn DocsView(page: DocsPage, globals: ArgMap, project: Project) -> Element {
    let stories = page.stories().to_vec();
    let primary = page.primary();
    rsx! {
        article { class: "dxsb-docs",
            header { class: "dxsb-docs-head",
                p { class: "dxsb-docs-path", "{page.title()}" }
                h1 { class: "dxsb-docs-title", "{page.component()}" }
                if !page.description().is_empty() {
                    p { class: "dxsb-docs-lede",
                        Doc { text: page.description().to_string() }
                    }
                }
            }

            if let Some(def) = primary {
                PropsTable { story: def }
            } else {
                p { class: "dxsb-docs-note",
                    "No props table: every story here builds its own "
                    code { "Element" }
                    ", so there is no props type to introspect."
                }
            }

            section { class: "dxsb-docs-stories",
                h2 { class: "dxsb-docs-h2", "Stories" }
                for def in stories {
                    DocsStory {
                        key: "{def.id()}",
                        story: def,
                        globals: globals.clone(),
                        project,
                    }
                }
            }
        }
    }
}

/// One story on the docs page: its name, its `///`, the live example, its source.
#[component]
fn DocsStory(
    story: &'static StoryDef,
    globals: ArgMap,
    project: Project,
) -> Element {
    let id = story.id();
    rsx! {
        section { class: "dxsb-docs-story",
            h3 { class: "dxsb-docs-h3", "{story.name()}" }
            if !story.docs().is_empty() {
                p { class: "dxsb-docs-blurb", Doc { text: story.docs().to_string() } }
            }
            div { class: "dxsb-docs-example",
                // A keyed list of exactly one, for the same reason the canvas
                // uses one: Dioxus honours `key` only when diffing a list, and
                // each example must own its hooks rather than share this scope's.
                for def in [story] {
                    StoryHost {
                        key: "{def.id()}",
                        story: def,
                        // Defaults, deliberately. The URL's args belong to the
                        // story you selected, and laying them over every example
                        // on the page would make the documentation depend on
                        // which control you last dragged.
                        args: ArgMap::new(),
                        globals: globals.clone(),
                        project,
                        // The canvas is what the shell is watching. A docs page
                        // renders many stories at once, and letting each one
                        // announce itself would leave the status bar naming
                        // whichever happened to be last and the controls panel
                        // seeded from the wrong story.
                        reporting: false,
                        // What lets a project decorator tell a docs example
                        // from a canvas. Without it, a decorator painting
                        // `min-height:100vh` — which is exactly right on the
                        // canvas — makes every example on this page a screen
                        // tall. See `StoryView`.
                        view: StoryView::Docs,
                    }
                }
            }
            if !story.source().is_empty() {
                details { class: "dxsb-docs-source",
                    summary { "Show code" }
                    pre { code { "{story.source()}" } }
                }
            }
            p { class: "dxsb-docs-id", "{id}" }
        }
    }
}

/// The props table: one row per prop, straight off `#[derive(Controls)]`.
///
/// The `Default` column is the story's own evaluated props, which is why this
/// is a component with a hook rather than a formatting function —
/// [`StoryDef::base_args`] needs a live scope, and calling it once per mount is
/// what keeps it from allocating an `EventHandler` on every render.
#[component]
fn PropsTable(story: &'static StoryDef) -> Element {
    let rows: &'static [ArgType] = story.arg_types();
    let defaults = use_hook(|| story.base_args());
    rsx! {
        section { class: "dxsb-docs-props",
            h2 { class: "dxsb-docs-h2", "Props" }
            table { class: "dxsb-proptable",
                thead {
                    tr {
                        th { "Name" }
                        th { "Type" }
                        th { "Default" }
                        th { "Description" }
                    }
                }
                tbody {
                    for arg in rows.iter() {
                        tr { key: "{arg.name}",
                            td { class: "dxsb-propcell-name",
                                span { class: "dxsb-propname", "{arg.name}" }
                                if !arg.required {
                                    span { class: "dxsb-optional", "?" }
                                }
                            }
                            td { code { class: "dxsb-proptype", "{arg.ty}" } }
                            td { class: "dxsb-propcell-default",
                                {default_cell(arg, &defaults)}
                            }
                            td { class: "dxsb-propcell-docs",
                                if arg.docs.is_empty() {
                                    span { class: "dxsb-muted", "—" }
                                } else {
                                    Doc { text: arg.docs.to_string() }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// What the `Default` column shows for one prop.
///
/// An event handler has no value to print and a skipped field was never seeded,
/// so both say what they are instead of showing an empty cell that reads as a
/// missing default.
fn default_cell(arg: &'static ArgType, defaults: &ArgMap) -> Element {
    match arg.control {
        Control::Action => rsx! { span { class: "dxsb-muted", "handler" } },
        Control::None => rsx! { span { class: "dxsb-muted", "—" } },
        _ => match defaults.get(arg.name) {
            Some(value) => {
                let text = value.as_text();
                if text.is_empty() {
                    rsx! { span { class: "dxsb-muted", "—" } }
                } else {
                    rsx! { code { class: "dxsb-propdefault", "{text}" } }
                }
            }
            None => rsx! { span { class: "dxsb-muted", "—" } },
        },
    }
}
