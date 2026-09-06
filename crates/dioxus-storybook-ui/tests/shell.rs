//! The manager shell, rendered for real.
//!
//! The browser layer is a no-op off `wasm32`, so the whole shell renders on the
//! host and its markup can be asserted on. That covers the sidebar tree, the
//! search filter, the landing-story rule and the empty states — everything
//! except the parts that genuinely need a DOM (focus, history).

use dioxus::prelude::*;
use dioxus_ssr::render;
use dioxus_storybook_core::{Registry, StoryDef};
use dioxus_storybook_ui::{Storybook, StorybookProps};

static PRIMARY: StoryDef = StoryDef::new("Forms/Button", "Primary", |_| {
    rsx! { p { class: "story", "primary-body" } }
});
static DANGER: StoryDef = StoryDef::new("Forms/Button", "Danger", |_| {
    rsx! { p { class: "story", "danger-body" } }
});
static TEXT_INPUT: StoryDef = StoryDef::new("Forms/Input", "Text", |_| {
    rsx! { p { class: "story", "input-body" } }
});
static CARD: StoryDef = StoryDef::new("Layout/Card", "Default", |_| {
    rsx! { p { class: "story", "card-body" } }
});

static ALL: &[&StoryDef] = &[&PRIMARY, &DANGER, &TEXT_INPUT, &CARD];
static NONE: &[&StoryDef] = &[];

fn shell(stories: &'static [&'static StoryDef]) -> String {
    let registry = Registry::new(stories);
    let mut dom = VirtualDom::new_with_props(Storybook, StorybookProps { registry });
    dom.rebuild_in_place();
    render(&dom)
}

#[test]
fn the_sidebar_shows_every_group_and_story() {
    let html = shell(ALL);
    for expected in [
        "Forms", "Button", "Input", "Layout", "Card", "Primary", "Danger", "Text", "Default",
    ] {
        assert!(html.contains(expected), "sidebar is missing {expected}");
    }
}

#[test]
fn the_first_story_renders_without_waiting_for_an_effect() {
    // The preview resolves its own landing story, so the first paint is the
    // story itself rather than a placeholder.
    let html = shell(ALL);
    assert!(html.contains("primary-body"), "got: {html}");
    assert!(!html.contains("Select a story from the sidebar"));
}

#[test]
fn the_toolbar_shows_the_selected_storys_breadcrumb_and_id() {
    let html = shell(ALL);
    assert!(html.contains("forms-button--primary"), "got: {html}");
}

#[test]
fn the_status_bar_counts_the_registry() {
    let html = shell(ALL);
    assert!(html.contains("4 stories"), "got: {html}");
}

#[test]
fn an_empty_registry_explains_itself_instead_of_rendering_blank() {
    let html = shell(NONE);
    assert!(
        html.contains("No stories registered"),
        "an empty sidebar is almost always a wiring mistake, and should say so: {html}"
    );
    assert!(html.contains("dioxus_storybook_build::index"));
}

#[test]
fn the_search_box_is_present_and_addressable() {
    let html = shell(ALL);
    assert!(html.contains("dxsb-search"));
}

#[test]
fn story_bodies_are_invoked_in_scope() {
    // A story that builds an EventHandler panics unless it runs inside a live
    // Dioxus scope. Rendering the shell is the proof that it does.
    static HANDLER: StoryDef = StoryDef::new("Forms/Button", "Handler", |_| {
        let handler = EventHandler::new(|_: MouseEvent| {});
        rsx! { button { onclick: move |e| handler.call(e), "ok" } }
    });
    static WITH_HANDLER: &[&StoryDef] = &[&HANDLER];

    let html = shell(WITH_HANDLER);
    assert!(html.contains("ok"), "got: {html}");
}
