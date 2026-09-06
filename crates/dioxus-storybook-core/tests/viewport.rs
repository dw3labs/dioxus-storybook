//! The viewport addon's one decision: how big is the canvas.
//!
//! Everything here is about [`viewport::resolve`], which both halves of the app
//! call — the manager to size the frame, the preview to fill in
//! `StoryContext::viewport`. If the two ever disagreed, this is the function
//! that would have let them.

use dioxus_storybook_core::viewport::{self, RESPONSIVE, ROTATED_GLOBAL, VIEWPORT_GLOBAL};
use dioxus_storybook_core::{
    ArgMap, ArgValue, ParamValue, Parameters, Project, ResolvedParameters, Viewport,
};

static SIZES: &[Viewport] = &[
    Viewport::new("phone", "Phone", 390, 844),
    Viewport::new("desk", "Desk", 1440, 900),
];

fn nothing() -> ResolvedParameters {
    ResolvedParameters::default()
}

/// A story-level `viewport` parameter, which is the level that wins.
fn story_wants(name: &'static str) -> ResolvedParameters {
    static PHONE: &[(&str, ParamValue)] = &[("viewport", ParamValue::Str("phone"))];
    static DESK: &[(&str, ParamValue)] = &[("viewport", ParamValue::Str("desk"))];
    static TYPO: &[(&str, ParamValue)] = &[("viewport", ParamValue::Str("phne"))];
    let level = match name {
        "phone" => Parameters::from_static(PHONE),
        "desk" => Parameters::from_static(DESK),
        _ => Parameters::from_static(TYPO),
    };
    ResolvedParameters::new(Parameters::new(), Parameters::new(), level)
}

fn picked(name: &str) -> ArgMap {
    ArgMap::new().with(VIEWPORT_GLOBAL, ArgValue::Variant(name.into()))
}

#[test]
fn nothing_selected_and_nothing_asked_for_is_responsive() {
    assert!(viewport::resolve(SIZES, &ArgMap::new(), nothing()).is_none());
}

#[test]
fn a_selection_gives_the_declared_size() {
    let resolved = viewport::resolve(SIZES, &picked("phone"), nothing()).expect("no viewport");
    assert_eq!(resolved.name(), "phone");
    assert_eq!((resolved.width(), resolved.height()), (390, 844));
    assert!(!resolved.rotated());
}

#[test]
fn a_story_can_ask_to_open_at_a_size() {
    // The half of the addon that is a *parameter*: static, per level, and the
    // thing that actually varies per story. A mobile nav drawer should open on
    // a phone without anybody touching the toolbar first.
    let resolved = viewport::resolve(SIZES, &ArgMap::new(), story_wants("phone")).unwrap();
    assert_eq!(resolved.name(), "phone");
}

#[test]
fn a_selection_beats_what_the_story_asked_for() {
    let resolved = viewport::resolve(SIZES, &picked("desk"), story_wants("phone")).unwrap();
    assert_eq!(resolved.name(), "desk");
}

#[test]
fn responsive_is_a_value_and_not_merely_an_absence() {
    // The reason `RESPONSIVE` exists at all. A story that asks for a phone
    // would otherwise be impossible to look at full width: removing the
    // selection just hands the decision back to the parameter.
    let selected = picked(RESPONSIVE);
    assert!(viewport::resolve(SIZES, &selected, story_wants("phone")).is_none());
    assert!(viewport::resolve(SIZES, &ArgMap::new(), story_wants("phone")).is_some());
}

#[test]
fn responsive_only_needs_saying_when_it_overrides_something() {
    // What keeps `viewport:responsive` out of a link that does not need it —
    // and keeps an empty selection map meaning "nothing has been changed",
    // which is the question the reset button asks.
    assert!(!viewport::responsive_needs_saying(SIZES, &picked("desk"), nothing()));
    assert!(viewport::responsive_needs_saying(SIZES, &picked("desk"), story_wants("phone")));
}

#[test]
fn rotation_swaps_the_two_numbers_and_only_the_two_numbers() {
    let selected = picked("phone").with(ROTATED_GLOBAL, ArgValue::Bool(true));
    let resolved = viewport::resolve(SIZES, &selected, nothing()).unwrap();
    assert!(resolved.rotated());
    assert_eq!((resolved.width(), resolved.height()), (844, 390));
    // The declaration is untouched: the picker still lists a 390×844 phone.
    assert_eq!(
        (resolved.viewport().width(), resolved.viewport().height()),
        (390, 844)
    );
}

#[test]
fn a_viewport_this_build_does_not_have_is_a_responsive_canvas_not_a_crash() {
    // Both routes in: a pasted link naming a viewport that has since been
    // removed, and a parameter with a typo in it. Neither is worth failing a
    // page load over — the same rule `canvas_layout` follows for `layout`.
    assert!(viewport::resolve(SIZES, &picked("watch"), nothing()).is_none());
    assert!(viewport::resolve(SIZES, &ArgMap::new(), story_wants("typo")).is_none());
}

#[test]
fn a_pasted_selection_resolves_the_same_as_a_picked_one() {
    // The URL codec is untyped by design: `?globals=viewport:phone` decodes as
    // `Text`, while the picker sets `Variant`. Anything comparing the two shapes
    // without coercing works when clicked and fails when pasted.
    let pasted = dioxus_storybook_core::url::decode_args("viewport:phone;viewport-rotated:!true");
    let resolved = viewport::resolve(SIZES, &pasted, nothing()).expect("no viewport");
    assert_eq!(resolved.name(), "phone");
    assert_eq!((resolved.width(), resolved.height()), (844, 390));
}

#[test]
fn a_project_with_no_viewports_has_no_viewport() {
    // `with_viewports(&[])` is how the picker is removed altogether, so it must
    // also pin the canvas responsive — including for a story that asks.
    assert!(viewport::resolve(&[], &picked("phone"), story_wants("phone")).is_none());
    assert!(Project::new().with_viewports(&[]).viewports().is_empty());
}

#[test]
fn every_storybook_has_viewports_unless_it_says_otherwise() {
    // `Project::default()` and `Project::new()` must be the same project: the
    // shell takes the project as `#[props(default)]`, so a derived `Default`
    // would silently give a storybook mounted without one an empty picker.
    assert_eq!(Project::default(), Project::new());
    assert_eq!(Project::new().viewports(), viewport::DEFAULT_VIEWPORTS);
    assert!(!viewport::DEFAULT_VIEWPORTS.is_empty());
}

#[test]
fn the_built_in_names_are_url_safe_and_unique() {
    // They travel in a query string. A name needing percent-encoding would
    // still work and would still be unreadable in a shared link.
    let mut seen: Vec<&str> = Vec::new();
    for view in viewport::DEFAULT_VIEWPORTS {
        assert!(
            view.name()
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "not url-safe: {}",
            view.name()
        );
        assert_ne!(view.name(), RESPONSIVE, "a viewport shadowed the reserved name");
        assert!(!seen.contains(&view.name()), "duplicate: {}", view.name());
        assert!(view.width() > 0 && view.height() > 0);
        seen.push(view.name());
    }
    assert!(viewport::find(viewport::DEFAULT_VIEWPORTS, "tablet").is_some());
}
