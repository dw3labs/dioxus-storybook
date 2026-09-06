//! The whole authoring surface, exercised the way a user meets it: declare a
//! component, declare stories, then render them.
//!
//! These render through a real `VirtualDom`, not a mock. That distinction has
//! already mattered once on this project — the M0 `inventory` spike compiled
//! cleanly and was still wrong — so "it builds" is not accepted as "it works".

use dioxus::prelude::*;
use dioxus_storybook::prelude::*;
use dioxus_storybook::{ArgValue, Control, Registry, StoryDef};

// ---------------------------------------------------------------- component

/// How much visual weight a button carries.
#[derive(Clone, Copy, PartialEq, Debug, ControlEnum)]
pub enum ButtonVariant {
    Primary,
    Danger,
}

/// A button.
#[derive(Props, Clone, PartialEq, Controls)]
pub struct ButtonProps {
    /// Text shown inside the button.
    pub label: String,
    /// Visual emphasis of the button.
    pub variant: ButtonVariant,
    /// Relative size multiplier.
    #[control(range(min = 0.5, max = 3.0, step = 0.25))]
    pub scale: f32,
    /// Whether the button rejects interaction.
    pub disabled: bool,
    /// Optional tooltip shown on hover.
    pub tooltip: Option<String>,
    /// Fired on click.
    pub onclick: EventHandler<MouseEvent>,
}

#[component]
pub fn Button(props: ButtonProps) -> Element {
    let variant = format!("{:?}", props.variant).to_lowercase();
    let scale = props.scale;
    rsx! {
        button {
            class: "btn btn-{variant}",
            disabled: props.disabled,
            style: "font-size:{scale}rem",
            onclick: move |e| props.onclick.call(e),
            "{props.label}"
        }
    }
}

// ------------------------------------------------------------------ stories

story_meta! {
    title: "Forms/Button",
    component: Button,
    tags: ["autodocs"],
}

fn base() -> ButtonProps {
    ButtonProps {
        label: "Click me".into(),
        variant: ButtonVariant::Primary,
        scale: 1.0,
        disabled: false,
        tooltip: None,
        onclick: EventHandler::new(|_| {}),
    }
}

#[story]
fn primary() -> ButtonProps {
    base()
}

#[story]
fn danger() -> ButtonProps {
    ButtonProps {
        variant: ButtonVariant::Danger,
        label: "Delete".into(),
        ..base()
    }
}

#[story(name = "Two Buttons")]
fn two_buttons() -> Element {
    rsx! {
        div { class: "row",
            Button { ..base() }
            Button { label: "Cancel".into(), ..base() }
        }
    }
}

static ALL: &[&StoryDef] = &[&PRIMARY, &DANGER, &TWO_BUTTONS];

// ------------------------------------------------------------------ harness

#[component]
fn Harness(def: &'static StoryDef, args: ArgMap) -> Element {
    // Story bodies are fn pointers precisely so they can be invoked *here*,
    // inside a live scope, where `rsx!` and `EventHandler::new` are legal.
    rsx! { div { id: "root", {def.render(&args)} } }
}

fn render(def: &'static StoryDef, args: ArgMap) -> String {
    let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { def, args });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// Run `f` inside a live Dioxus scope.
///
/// Anything that evaluates a story's props needs one, because `EventHandler::new`
/// panics without it — a *scope*, note, not merely a runtime.
fn in_scope<T>(f: impl FnOnce() -> T) -> T {
    let mut dom = VirtualDom::new(|| rsx! { div {} });
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, f)
}

// -------------------------------------------------------------------- tests

#[test]
fn story_ids_come_from_the_meta_title_and_the_fn_name() {
    assert_eq!(PRIMARY.id(), "forms-button--primary");
    assert_eq!(DANGER.id(), "forms-button--danger");
    assert_eq!(TWO_BUTTONS.id(), "forms-button--two-buttons");
    assert_eq!(PRIMARY.name(), "Primary");
    assert_eq!(TWO_BUTTONS.name(), "Two Buttons", "name = \"...\" wins");
    assert_eq!(PRIMARY.title(), "Forms/Button");
}

#[test]
fn tags_are_inherited_from_the_meta() {
    assert!(PRIMARY.has_tag("autodocs"));
    assert!(TWO_BUTTONS.has_tag("autodocs"));
}

#[test]
fn a_story_renders_its_own_props_with_no_args() {
    let html = render(&PRIMARY, ArgMap::new());
    assert!(html.contains("Click me"), "got: {html}");
    assert!(html.contains("btn-primary"), "got: {html}");

    let html = render(&DANGER, ArgMap::new());
    assert!(html.contains("Delete"), "got: {html}");
    assert!(html.contains("btn-danger"), "got: {html}");
}

#[test]
fn args_overlay_the_storys_props() {
    let args = ArgMap::new()
        .with("label", ArgValue::Text("Overridden".into()))
        .with("variant", ArgValue::Variant("Danger".into()))
        .with("scale", ArgValue::Num(2.0))
        .with("disabled", ArgValue::Bool(true));

    let html = render(&PRIMARY, args);
    assert!(html.contains("Overridden"), "got: {html}");
    assert!(html.contains("btn-danger"), "enum override applied: {html}");
    assert!(html.contains("font-size:2rem"), "number override: {html}");
    assert!(html.contains("disabled"), "bool override: {html}");
}

#[test]
fn unset_args_keep_the_storys_typed_defaults() {
    let args = ArgMap::new().with("label", ArgValue::Text("Only the label".into()));
    let html = render(&DANGER, args);
    assert!(html.contains("Only the label"));
    assert!(
        html.contains("btn-danger"),
        "variant was not supplied, so the story's own value survives: {html}"
    );
}

#[test]
fn garbage_args_fall_back_instead_of_panicking() {
    // Args arrive from a URL, so this is the normal case, not the edge case.
    let args = ArgMap::new()
        .with("scale", ArgValue::Text("not-a-number".into()))
        .with("variant", ArgValue::Variant("Nonexistent".into()))
        .with("nosuchfield", ArgValue::Bool(true));

    let html = render(&PRIMARY, args);
    assert!(html.contains("font-size:1rem"), "bad number ignored: {html}");
    assert!(html.contains("btn-primary"), "bad variant ignored: {html}");
}

#[test]
fn the_element_form_renders_without_controls() {
    let html = render(&TWO_BUTTONS, ArgMap::new());
    assert!(html.contains("Click me"));
    assert!(html.contains("Cancel"));
    assert!(
        TWO_BUTTONS.arg_types().is_empty(),
        "an Element story has no props type to introspect"
    );
    assert!(in_scope(|| TWO_BUTTONS.base_args()).is_empty());
}

#[test]
fn arg_types_carry_doc_comments_types_and_widgets() {
    let types = PRIMARY.arg_types();
    let by_name = |n: &str| types.iter().find(|t| t.name == n).expect(n);

    let label = by_name("label");
    assert_eq!(label.docs, "Text shown inside the button.");
    assert_eq!(label.ty, "String");
    assert_eq!(label.control, Control::Text);
    assert!(label.required);

    assert_eq!(
        by_name("variant").control,
        Control::Select {
            options: &["Primary", "Danger"]
        }
    );
    assert_eq!(
        by_name("scale").control,
        Control::Range {
            min: 0.5,
            max: 3.0,
            step: 0.25
        },
        "#[control(range(..))] beats the inferred Number widget"
    );
    assert_eq!(by_name("disabled").control, Control::Toggle);

    let tooltip = by_name("tooltip");
    assert!(!tooltip.required, "Option<T> is optional");

    let onclick = by_name("onclick");
    assert_eq!(onclick.control, Control::Action);
    assert!(!onclick.is_dynamic(), "handlers are not overlaid");
}

#[test]
fn base_args_seed_only_the_controllable_fields() {
    let seeded = in_scope(|| PRIMARY.base_args());
    assert_eq!(
        seeded.get("label"),
        Some(&ArgValue::Text("Click me".into()))
    );
    assert_eq!(
        seeded.get("variant"),
        Some(&ArgValue::Variant("Primary".into()))
    );
    assert_eq!(seeded.get("tooltip"), Some(&ArgValue::Null));
    assert!(
        seeded.get("onclick").is_none(),
        "an EventHandler has no dynamic value to seed"
    );
}

#[test]
fn a_registry_over_these_stories_indexes_and_searches_them() {
    let registry = Registry::new(ALL);
    assert_eq!(registry.len(), 3);
    assert_eq!(
        registry.get("forms-button--danger").map(StoryDef::name),
        Some("Danger")
    );
    let tree = registry.tree();
    assert_eq!(tree.len(), 1, "all three share one Forms group");
    assert_eq!(registry.search("danger").len(), 1);
}
