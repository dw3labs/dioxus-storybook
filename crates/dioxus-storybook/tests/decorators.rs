//! Decorators and the three-level parameter merge.
//!
//! Both answer the same question — "what does a story inherit, and from where?"
//! — so they are tested together. The levels are project, component and story,
//! and the rule in both cases is that the innermost one wins: a story overrides
//! its component, which overrides the project. Decorators differ only in that
//! nothing is overridden; every level's decorators apply, and the order they
//! apply in is what makes them composable.

use dioxus_storybook::prelude::*;
use dioxus_storybook::{Decorator, ParamValue, Project, StoryContext, StoryDef};

// ------------------------------------------------------------------ merging

static PROJECT_PARAMS: &[(&str, ParamValue)] = &[
    ("layout", ParamValue::Str("padded")),
    ("theme", ParamValue::Str("light")),
    ("docs", ParamValue::Bool(true)),
];
static COMPONENT_PARAMS: &[(&str, ParamValue)] = &[
    ("layout", ParamValue::Str("fullscreen")),
    ("width", ParamValue::Num(320.0)),
];
static STORY_PARAMS: &[(&str, ParamValue)] = &[("layout", ParamValue::Str("centered"))];

static META: Meta = Meta::new("Forms/Button", "Button")
    .with_parameters(Parameters::from_static(COMPONENT_PARAMS));

static STORY: StoryDef = StoryDef::new("Forms/Button", "Primary", |_| {
    rsx! { p { "body" } }
})
.with_parameters(Parameters::from_static(STORY_PARAMS))
.with_meta(META);

fn project() -> Project {
    Project::new().with_parameters(Parameters::from_static(PROJECT_PARAMS))
}

#[test]
fn the_innermost_level_that_sets_a_key_wins() {
    let params = STORY.resolved_parameters(project());
    assert_eq!(params.str("layout"), Some("centered"), "the story should win");
    assert_eq!(params.num("width"), Some(320.0), "the component should fill the gap");
    assert_eq!(params.str("theme"), Some("light"), "the project is the floor");
    assert_eq!(params.get("absent"), None);
}

#[test]
fn a_story_with_no_parameters_of_its_own_still_inherits_both_levels() {
    static BARE: StoryDef =
        StoryDef::new("Forms/Button", "Bare", |_| rsx! { p { "b" } }).with_meta(META);
    let params = BARE.resolved_parameters(project());
    assert_eq!(params.str("layout"), Some("fullscreen"));
    assert_eq!(params.str("theme"), Some("light"));
}

#[test]
fn a_story_with_no_meta_at_all_still_sees_the_project() {
    static LOOSE: StoryDef = StoryDef::new("Loose/Story", "Default", |_| rsx! { p { "l" } });
    let params = LOOSE.resolved_parameters(project());
    assert_eq!(params.str("theme"), Some("light"));
}

#[test]
fn reading_a_parameter_as_the_wrong_type_yields_the_default_rather_than_a_panic() {
    // A typo three levels up must not be able to take an addon down.
    let params = STORY.resolved_parameters(project());
    assert_eq!(params.num("layout"), None);
    assert_eq!(params.str("width"), None);
    assert!(params.flag("layout", true));
    assert!(!params.flag("layout", false));
    assert!(params.flag("docs", false), "a real bool still reads");
}

#[test]
fn iterating_lists_each_key_once_with_its_winning_value() {
    let params = STORY.resolved_parameters(project());
    let all: Vec<_> = params.iter().collect();
    assert_eq!(all.len(), 4, "expected layout, width, theme, docs — got {all:?}");
    assert!(all.contains(&("layout", ParamValue::Str("centered"))));
    assert!(!all.iter().any(|(k, v)| *k == "layout" && *v == ParamValue::Str("padded")));
}

// --------------------------------------------------------------- decorating

/// Rendering has to happen inside a live Dioxus scope — decorators use `rsx!`
/// exactly as story bodies do.
fn render_in_scope(def: &'static StoryDef, project: Project, args: ArgMap) -> String {
    #[component]
    fn Harness(def: &'static StoryDef, project: Project, args: ArgMap) -> Element {
        def.render_decorated(project, &args)
    }
    let mut dom =
        VirtualDom::new_with_props(Harness, HarnessProps { def, project, args });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

static PROJECT_DECORATORS: &[Decorator] = &[|_ctx: &StoryContext, story: Element| {
    rsx! { div { class: "project", {story} } }
}];
static COMPONENT_DECORATORS: &[Decorator] = &[|_ctx: &StoryContext, story: Element| {
    rsx! { div { class: "component", {story} } }
}];
static STORY_DECORATORS: &[Decorator] = &[|_ctx: &StoryContext, story: Element| {
    rsx! { div { class: "story-level", {story} } }
}];

static DECORATED_META: Meta =
    Meta::new("Forms/Button", "Button").with_decorators(COMPONENT_DECORATORS);

static DECORATED: StoryDef = StoryDef::new("Forms/Button", "Primary", |_| {
    rsx! { p { class: "body", "the story" } }
})
.with_decorators(STORY_DECORATORS)
.with_meta(DECORATED_META);

#[test]
fn the_project_wraps_the_component_which_wraps_the_story() {
    let html = render_in_scope(
        &DECORATED,
        Project::new().with_decorators(PROJECT_DECORATORS),
        ArgMap::new(),
    );
    let expected = concat!(
        r#"<div class="project">"#,
        r#"<div class="component">"#,
        r#"<div class="story-level">"#,
        r#"<p class="body">the story</p>"#,
        "</div></div></div>",
    );
    assert_eq!(html, expected, "decorators nested in the wrong order");
}

#[test]
fn a_story_with_no_decorators_renders_exactly_what_it_returned() {
    static PLAIN: StoryDef =
        StoryDef::new("Forms/Button", "Plain", |_| rsx! { p { "plain" } });
    assert_eq!(render_in_scope(&PLAIN, Project::new(), ArgMap::new()), "<p>plain</p>");
}

/// Within one level the first decorator in the slice is the innermost, so a
/// list reads outermost-last — the same direction as the level order.
static TWO: &[Decorator] = &[
    |_ctx: &StoryContext, story: Element| rsx! { div { class: "inner", {story} } },
    |_ctx: &StoryContext, story: Element| rsx! { div { class: "outer", {story} } },
];

#[test]
fn within_one_level_the_first_decorator_is_the_innermost() {
    static PAIRED: StoryDef =
        StoryDef::new("A/B", "C", |_| rsx! { p { "x" } }).with_decorators(TWO);
    assert_eq!(
        render_in_scope(&PAIRED, Project::new(), ArgMap::new()),
        r#"<div class="outer"><div class="inner"><p>x</p></div></div>"#
    );
}

// ------------------------------------------------------------- the context

static CONTEXT_REPORTER: &[Decorator] = &[|ctx: &StoryContext, _story: Element| {
    let id = ctx.id().to_string();
    let layout = ctx.parameters().str("layout").unwrap_or("none");
    let label = ctx.args().get("label").cloned();
    let label = match label {
        Some(ArgValue::Text(t)) => t,
        _ => "none".into(),
    };
    rsx! { p { "{id}|{layout}|{label}|{ctx.name()}" } }
}];

#[test]
fn a_decorator_is_told_which_story_it_is_wrapping_and_with_what() {
    static REPORTED: StoryDef = StoryDef::new("Forms/Button", "With Icon", |_| {
        rsx! { p { "ignored" } }
    })
    .with_decorators(CONTEXT_REPORTER)
    .with_parameters(Parameters::from_static(STORY_PARAMS));

    let args = ArgMap::new().with("label", ArgValue::Text("Hey".into()));
    let html = render_in_scope(&REPORTED, project(), args);
    assert_eq!(html, "<p>forms-button--with-icon|centered|Hey|With Icon</p>");
}

// ------------------------------------------------- the same thing, authored

/// Everything above is the builder API. This is what a user actually writes,
/// and the point of testing both is that the macros are the only way a real
/// story ever gets a decorator or a parameter attached.
mod authored {
    use dioxus_storybook::prelude::*;
    use dioxus_storybook::{ParamValue, Project, StoryContext};

    #[derive(Props, Clone, PartialEq, Controls)]
    pub struct ChipProps {
        /// The chip's text.
        pub label: String,
    }

    #[component]
    pub fn Chip(props: ChipProps) -> Element {
        rsx! { span { class: "chip", "{props.label}" } }
    }

    fn framed(_ctx: &StoryContext, story: Element) -> Element {
        rsx! { div { class: "frame", {story} } }
    }

    story_meta! {
        title: "Data/Chip",
        component: Chip,
        parameters: { layout: "padded", width: 320, docs: false },
        decorators: [framed],
    }

    fn highlight(_ctx: &StoryContext, story: Element) -> Element {
        rsx! { mark { {story} } }
    }

    #[story]
    fn plain() -> ChipProps {
        ChipProps { label: "Tag".into() }
    }

    #[story(parameters { layout: "centered" }, decorators = [highlight])]
    fn special() -> ChipProps {
        ChipProps { label: "Special".into() }
    }

    #[test]
    fn story_meta_parameters_reach_every_story_in_the_module() {
        let params = PLAIN.resolved_parameters(Project::new());
        assert_eq!(params.str("layout"), Some("padded"));
        // An integer literal becomes the same `Num` a float would, so an addon
        // never has to ask which way the author wrote it.
        assert_eq!(params.num("width"), Some(320.0));
        assert_eq!(params.get("docs"), Some(ParamValue::Bool(false)));
    }

    #[test]
    fn a_story_parameter_overrides_its_components() {
        let params = SPECIAL.resolved_parameters(Project::new());
        assert_eq!(params.str("layout"), Some("centered"));
        assert_eq!(params.num("width"), Some(320.0), "the rest is still inherited");
    }

    #[test]
    fn decorators_from_both_macro_levels_apply_in_order() {
        // The keys of this test: `framed` came from `story_meta!` and so wraps
        // every story; `highlight` came from `#[story]` and wraps only its own,
        // inside the component's.
        assert_eq!(
            render(&PLAIN),
            r#"<div class="frame"><span class="chip">Tag</span></div>"#
        );
        assert_eq!(
            render(&SPECIAL),
            r#"<div class="frame"><mark><span class="chip">Special</span></mark></div>"#
        );
    }

    fn render(def: &'static dioxus_storybook::StoryDef) -> String {
        #[component]
        fn Harness(def: &'static dioxus_storybook::StoryDef) -> Element {
            def.render_decorated(Project::new(), &ArgMap::new())
        }
        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { def });
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }
}
