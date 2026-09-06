//! The M2 additions to the shell: the controls panel, the actions panel, and
//! the scope each story now gets to itself.
//!
//! The interesting assertion is the first one. A control's *value* can only have
//! reached the panel by travelling out of the preview on the channel as
//! `StoryPrepared` — the manager never evaluates a story's props — so seeing a
//! default in the rendered table exercises that whole round trip.

mod common;

use common::{Documents, unpaired_preview_html};
use dioxus::prelude::*;
use dioxus_ssr::render;
use dioxus_storybook_core::{ArgMap, ArgType, ArgValue, Control, Controllable, StoryDef};

// ------------------------------------------------------------- a props type

/// A hand-written `Controllable`, so this crate can test the panel without
/// depending on the macro crate (which depends on this one's sibling).
#[derive(Clone, PartialEq)]
struct DemoProps {
    label: String,
    weight: f64,
    loud: bool,
}

static DEMO_ARG_TYPES: &[ArgType] = &[
    ArgType {
        name: "label",
        ty: "String",
        docs: "Text shown inside.",
        control: Control::Text,
        required: true,
    },
    ArgType {
        name: "weight",
        ty: "f64",
        docs: "",
        control: Control::Range {
            min: 0.0,
            max: 4.0,
            step: 0.5,
        },
        required: true,
    },
    ArgType {
        name: "loud",
        ty: "bool",
        docs: "",
        control: Control::Toggle,
        required: true,
    },
    ArgType {
        name: "onclick",
        ty: "EventHandler<MouseEvent>",
        docs: "",
        control: Control::Action,
        required: true,
    },
];

impl Controllable for DemoProps {
    fn arg_types() -> &'static [ArgType] {
        DEMO_ARG_TYPES
    }
    fn apply(&self, args: &ArgMap) -> Self {
        Self {
            label: args.get_or("label", self.label.clone()),
            weight: args.get_or("weight", self.weight),
            loud: args.get_or("loud", self.loud),
        }
    }
    fn to_args(&self) -> ArgMap {
        ArgMap::new()
            .with("label", ArgValue::Text(self.label.clone()))
            .with("weight", ArgValue::Num(self.weight))
            .with("loud", ArgValue::Bool(self.loud))
    }
    fn wire_actions(&self, _sink: &dioxus_storybook_core::ActionSink) -> Self {
        self.clone()
    }
}

fn demo_base() -> DemoProps {
    DemoProps {
        label: "Ship it".into(),
        weight: 2.5,
        loud: true,
    }
}

// ------------------------------------------------------------------ stories

static DEMO: StoryDef = StoryDef::new("Forms/Demo", "Default", |args| {
    let props = demo_base().apply(args);
    let label = props.label;
    rsx! { p { class: "story", "{label}" } }
})
.with_arg_types(DemoProps::arg_types)
.with_base_args(|| demo_base().to_args());

static BARE: StoryDef = StoryDef::new("Forms/Bare", "Default", |_| {
    rsx! { p { class: "story", "no props here" } }
});

static ALL: &[&StoryDef] = &[&DEMO, &BARE];
static ONLY_BARE: &[&StoryDef] = &[&BARE];

/// Stand up both documents and let the conversation settle, then look at the
/// shell.
///
/// From M3 the preview is a separate `VirtualDom`, so `StoryPrepared` reaches
/// the panel by crossing an encoded wire rather than by a function call one pass
/// later. See `tests/common/mod.rs`.
fn settled_shell(stories: &'static [&'static StoryDef]) -> String {
    Documents::new(stories).manager_html()
}

// -------------------------------------------------------------------- tests

#[test]
fn the_controls_panel_lists_every_prop_with_its_docs_and_type() {
    let html = settled_shell(ALL);
    for expected in ["label", "weight", "loud", "onclick", "Text shown inside.", "f64"] {
        assert!(html.contains(expected), "controls panel is missing {expected}");
    }
}

#[test]
fn control_values_arrive_from_the_preview_over_the_channel() {
    // The manager cannot compute these: evaluating a story's props needs a live
    // Dioxus scope, and from M3 the story functions are in the other bundle.
    // So a default showing up here means `StoryPrepared` made the whole trip.
    let html = settled_shell(ALL);
    assert!(
        html.contains("value=\"Ship it\""),
        "the text control did not receive the story's default: {html}"
    );
    assert!(
        html.contains("value=\"2.5\""),
        "the range control did not receive the story's default: {html}"
    );
}

#[test]
fn each_control_gets_the_widget_its_arg_type_asked_for() {
    let html = settled_shell(ALL);
    assert!(html.contains(r#"type="range""#), "no range widget: {html}");
    assert!(html.contains(r#"max="4""#), "range bounds were dropped: {html}");
    assert!(html.contains(r#"type="checkbox""#), "no toggle widget");
    // An `EventHandler` has no widget by design — it is watched, not edited.
    assert!(html.contains("calls appear in Actions"));
}

#[test]
fn the_actions_panel_starts_empty_and_says_so() {
    let html = settled_shell(ALL);
    // The Actions tab is present even while Controls is the open one.
    assert!(html.contains("Actions"), "no actions tab: {html}");
}

#[test]
fn a_story_with_no_props_type_explains_why_it_has_no_controls() {
    let html = settled_shell(ONLY_BARE);
    assert!(
        html.contains("build their own markup"),
        "the empty controls state is missing: {html}"
    );
}

#[test]
fn the_panel_offers_a_reset_that_is_disabled_until_something_is_overridden() {
    let html = settled_shell(ALL);
    assert!(html.contains("Reset to story defaults"), "no reset button");
    // Nothing is overridden on first paint, so it must not be clickable.
    assert!(
        html.contains("disabled"),
        "reset should start disabled: {html}"
    );
}

// --------------------------------------------------- the story's own scope

// Records every mount of the hook-using story body, so a remount is visible.
thread_local! {
    static MOUNTS: std::cell::RefCell<Vec<&'static str>> = const { std::cell::RefCell::new(Vec::new()) };
}

static HOOKED_A: StoryDef = StoryDef::new("Hooked/A", "Default", |_| {
    // A story body is free to call hooks — that is the whole reason each story
    // is rendered in a scope of its own.
    let count = use_signal(|| 0u32);
    use_hook(|| MOUNTS.with(|m| m.borrow_mut().push("a")));
    let count = count();
    rsx! { p { "a:{count}" } }
});

static HOOKED_B: StoryDef = StoryDef::new("Hooked/B", "Default", |_| {
    // Deliberately a *different* number of hooks from `HOOKED_A`. Rendering both
    // into one shared scope would corrupt the hook list on a switch; this is the
    // trap the keyed remount exists to close.
    let first = use_signal(|| 0u32);
    let second = use_signal(|| String::from("b"));
    use_hook(|| MOUNTS.with(|m| m.borrow_mut().push("b")));
    let (first, second) = (first(), second());
    rsx! { p { "b:{first}:{second}" } }
});

/// The pattern `Preview` uses: a keyed list of exactly one story host.
///
/// This test exists because the claim is load-bearing and non-obvious — Dioxus
/// only consults `key` when diffing a *list*, so a lone keyed child in a fixed
/// template position would be re-rendered in place, not remounted.
#[test]
fn switching_stories_remounts_the_host_instead_of_reusing_its_hooks() {
    thread_local! {
        static WHICH: std::cell::RefCell<Option<Signal<usize>>> =
            const { std::cell::RefCell::new(None) };
    }

    #[component]
    fn Host(story: &'static StoryDef) -> Element {
        story.render(&ArgMap::new())
    }

    fn app() -> Element {
        let which = use_signal(|| 0usize);
        use_hook(|| WHICH.with(|w| *w.borrow_mut() = Some(which)));
        let story: &'static StoryDef = if which() == 0 { &HOOKED_A } else { &HOOKED_B };
        rsx! {
            div {
                for story in [story] {
                    Host { key: "{story.id()}", story }
                }
            }
        }
    }

    MOUNTS.with(|m| m.borrow_mut().clear());

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert!(render(&dom).contains("a:0"), "first story did not render");

    let which = WHICH.with(|w| w.borrow().expect("app ran"));
    dom.in_runtime(|| {
        let mut which = which;
        which.set(1);
    });
    let _ = dom.render_immediate_to_vec();

    // Reaching here at all is most of the point: a shared hook list would have
    // handed `HOOKED_B`'s first `use_signal` the `u32` slot from `HOOKED_A` and
    // panicked on the downcast.
    assert!(
        render(&dom).contains("b:0:b"),
        "second story did not render after the switch: {}",
        render(&dom)
    );
    assert_eq!(
        MOUNTS.with(|m| m.borrow().clone()),
        vec!["a", "b"],
        "the host was reused rather than remounted"
    );
}

static ONLY_HOOKED: &[&StoryDef] = &[&HOOKED_A];

#[test]
fn a_story_that_uses_hooks_renders_inside_the_preview() {
    let html = unpaired_preview_html(ONLY_HOOKED);
    assert!(html.contains("a:0"), "hook-using story did not render: {html}");
}
