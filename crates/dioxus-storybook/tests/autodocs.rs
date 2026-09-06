//! M4: what a docs page is made of, and where each piece came from.
//!
//! Every fact on a generated docs page is read off the type by a macro — there
//! is no docgen pass and no sidecar file — so these tests are mostly assertions
//! that the *capture* happened. The one that is not is
//! [`the_source_snippet_is_the_code_as_written`], which is the whole reason
//! `#[story]` reads a span's source text rather than re-rendering its token
//! stream.

use dioxus::prelude::*;
use dioxus_storybook::prelude::*;
use dioxus_storybook::docs::{self, Entry};
use dioxus_storybook::{Registry, StoryDef};

// ---------------------------------------------------------------- component

/// A small status pill.
#[derive(Props, Clone, PartialEq, Controls)]
pub struct PillProps {
    /// The text inside the pill.
    pub text: String,
    /// Whether the pill is hollow.
    pub outline: bool,
}

#[component]
pub fn Pill(props: PillProps) -> Element {
    rsx! { span { class: "pill", "{props.text}" } }
}

/// A component nobody wrote prose for.
#[derive(Props, Clone, PartialEq, Controls)]
pub struct BareProps {
    /// A number.
    pub n: i32,
}

#[component]
pub fn Bare(props: BareProps) -> Element {
    rsx! { i { "{props.n}" } }
}

// -------------------------------------------------------- described component

mod described {
    use super::*;

    story_meta! {
        title: "Bits/Pill",
        component: Pill,
        description: "Prose written on the meta.",
        tags: ["autodocs"],
    }

    fn base() -> PillProps {
        PillProps { text: "New".into(), outline: false }
    }

    /// The ordinary one.
    #[story]
    pub fn solid() -> PillProps {
        // a comment, kept
        PillProps {
            outline: false,
            ..base()
        }
    }

    #[story]
    pub fn hollow() -> PillProps {
        PillProps { outline: true, ..base() }
    }

    /// Builds its own markup, so it has no props table.
    #[story(name = "A Row")]
    pub fn a_row() -> Element {
        rsx! { div { Pill { ..base() } } }
    }
}

// ------------------------------------------------------ undescribed component

mod undescribed {
    use super::*;

    story_meta! {
        title: "Bits/Bare",
        component: Bare,
    }

    #[story]
    pub fn plain() -> BareProps {
        BareProps { n: 1 }
    }
}

static ALL: &[&StoryDef] = &[
    &described::SOLID,
    &described::HOLLOW,
    &described::A_ROW,
    &undescribed::PLAIN,
];

fn registry() -> Registry {
    Registry::new(ALL)
}

// ------------------------------------------------------------------- capture

#[test]
fn a_story_carries_its_own_doc_comment() {
    assert_eq!(described::SOLID.docs(), "The ordinary one.");
    assert_eq!(
        described::HOLLOW.docs(),
        "",
        "a story with no doc comment says nothing rather than inventing something"
    );
}

#[test]
fn the_source_snippet_is_the_code_as_written() {
    let src = described::SOLID.source();
    assert!(
        src.contains("// a comment, kept"),
        "a comment survives, which a token stream would have dropped: {src:?}"
    );
    assert!(
        !src.starts_with('{') && !src.ends_with('}') || src.contains("PillProps {"),
        "the function's own braces are gone: {src:?}"
    );
    assert!(
        src.starts_with("// a comment, kept\nPillProps {"),
        "the body is dedented to column zero, leading blank line dropped: {src:?}"
    );
    assert!(
        src.contains("\n    outline: false,"),
        "the author's line breaks and indentation survive: {src:?}"
    );
    assert!(
        !src.contains("PillProps { outline : false"),
        "this is source text, not `quote!(..).to_string()`: {src:?}"
    );
}

#[test]
fn the_element_form_captures_its_rsx_too() {
    assert!(described::A_ROW.source().contains("rsx! {"));
    assert_eq!(described::A_ROW.arg_types(), &[], "no props type to introspect");
}

#[test]
fn a_description_comes_from_the_meta_and_falls_back_to_the_props_type() {
    assert_eq!(described::SOLID.meta().description(), "Prose written on the meta.");
    assert_eq!(described::SOLID.component_docs(), "A small status pill.");

    assert_eq!(undescribed::PLAIN.meta().description(), "");
    assert_eq!(undescribed::PLAIN.component_docs(), "A component nobody wrote prose for.");
}

// ------------------------------------------------------------------ resolving

#[test]
fn a_docs_id_is_the_component_slug_plus_a_suffix() {
    assert_eq!(docs::id_for("Bits/Pill"), "bits-pill--docs");
    assert_eq!(docs::id_for("Forms/Button"), "forms-button--docs");
}

#[test]
fn resolving_a_docs_id_collects_every_story_under_that_title() {
    let page = docs::resolve(registry(), Project::new(), "bits-pill--docs")
        .expect("Bits/Pill has stories");
    assert_eq!(page.title(), "Bits/Pill");
    assert_eq!(page.component(), "Pill");
    assert_eq!(page.description(), "Prose written on the meta.");
    let names: Vec<_> = page.stories().iter().map(|s| s.name()).collect();
    assert_eq!(names, ["Solid", "Hollow", "A Row"]);
}

#[test]
fn an_undescribed_component_falls_back_to_the_props_types_doc_comment() {
    let page = docs::resolve(registry(), Project::new(), "bits-bare--docs").unwrap();
    assert_eq!(page.description(), "A component nobody wrote prose for.");
}

#[test]
fn the_primary_story_is_the_first_one_that_has_a_props_table() {
    let page = docs::resolve(registry(), Project::new(), "bits-pill--docs").unwrap();
    assert_eq!(page.primary().map(StoryDef::name), Some("Solid"));
}

#[test]
fn an_unknown_slug_has_no_docs_page() {
    assert!(docs::resolve(registry(), Project::new(), "no-such-thing--docs").is_none());
    assert!(
        docs::resolve(registry(), Project::new(), "bits-pill--solid").is_none(),
        "a story id is not a docs id"
    );
}

// ---------------------------------------------------------------- the tag gate

#[test]
fn autodocs_defaults_to_every_component() {
    let project = Project::new();
    assert_eq!(project.autodocs(), Autodocs::Always);
    assert!(docs::resolve(registry(), project, "bits-bare--docs").is_some());
}

#[test]
fn tagged_mode_asks_for_the_autodocs_tag() {
    static TAGGED: Project = Project::new().with_autodocs(Autodocs::Tagged);
    assert!(
        docs::resolve(registry(), TAGGED, "bits-pill--docs").is_some(),
        "Bits/Pill declares tags: [\"autodocs\"]"
    );
    assert!(
        docs::resolve(registry(), TAGGED, "bits-bare--docs").is_none(),
        "Bits/Bare does not"
    );
}

#[test]
fn never_removes_every_docs_page() {
    static NEVER: Project = Project::new().with_autodocs(Autodocs::Never);
    assert!(docs::resolve(registry(), NEVER, "bits-pill--docs").is_none());
    assert!(docs::id_for_story(NEVER, &described::SOLID).is_none());
}

// -------------------------------------------------------------- the id space

#[test]
fn an_id_resolves_to_a_story_or_to_a_docs_page() {
    let reg = registry();
    let project = Project::new();
    assert!(matches!(
        docs::entry(reg, project, "bits-pill--solid"),
        Some(Entry::Story(_))
    ));
    assert!(matches!(
        docs::entry(reg, project, "bits-pill--docs"),
        Some(Entry::Docs(_))
    ));
    assert!(docs::entry(reg, project, "bits-pill--nope").is_none());
}

/// A story named "Docs" has the id a docs page would want. The author's story
/// is the one that should win, and there is no other way to reach it.
#[test]
fn a_real_story_wins_a_collision_with_a_docs_id() {
    mod collide {
        use super::*;
        story_meta! { title: "Bits/Clash", component: Bare }

        #[story(name = "Docs")]
        pub fn docs_story() -> BareProps {
            BareProps { n: 7 }
        }
    }
    static CLASH: &[&StoryDef] = &[&collide::DOCS_STORY];
    let reg = Registry::new(CLASH);
    assert_eq!(collide::DOCS_STORY.id(), "bits-clash--docs");
    assert!(matches!(
        docs::entry(reg, Project::new(), "bits-clash--docs"),
        Some(Entry::Story(_))
    ));
}

#[test]
fn the_canvas_button_lands_on_the_first_story_of_the_component() {
    assert_eq!(
        docs::canvas_id_for(registry(), "Bits/Pill").as_deref(),
        Some("bits-pill--solid")
    );
    assert!(docs::canvas_id_for(registry(), "Bits/Nothing").is_none());
}

// ----------------------------------------------------------------- StoryView

/// A decorator that paints the whole surface has to size itself differently on
/// the two, and this is the only thing that tells it which one it is on.
#[test]
fn a_decorator_is_told_which_surface_it_is_on() {
    static PROBE: &[Decorator] = &[|ctx: &StoryContext, story: Element| {
        let seen = match ctx.view() {
            StoryView::Docs => "on-docs",
            _ => "on-canvas",
        };
        rsx! { div { class: "{seen}", {story} } }
    }];
    static PROJECT: Project = Project::new().with_decorators(PROBE);

    // A component rather than a closure: `VirtualDom::new` takes a `fn`
    // pointer, so the view has to arrive as a prop.
    #[component]
    fn Probe(view: StoryView) -> Element {
        described::SOLID.render_decorated(PROJECT, &ArgMap::new(), &ArgMap::new(), view)
    }

    let render = |view: StoryView| {
        let mut dom = VirtualDom::new_with_props(Probe, ProbeProps { view });
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    };

    assert!(render(StoryView::Canvas).contains(r#"class="on-canvas""#));
    assert!(render(StoryView::Docs).contains(r#"class="on-docs""#));
}

#[test]
fn the_canvas_is_the_default_view() {
    assert_eq!(StoryView::default(), StoryView::Canvas);
}
