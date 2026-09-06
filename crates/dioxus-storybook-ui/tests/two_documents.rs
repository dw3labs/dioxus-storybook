//! The M3 split: the manager and the preview as two documents.
//!
//! Everything here is a claim about the *architecture* rather than about any
//! one feature — that the two halves are genuinely separable, that they only
//! ever talk through the wire, and that neither depends on hearing its own
//! voice. See `tests/common/mod.rs` for how the pair is stood up.

mod common;

use common::Documents;
use dioxus::prelude::*;
use dioxus_storybook_core::{
    ArgMap, ArgValue, Controllable, Decorator, Event, GlobalType, StoryContext, StoryDef,
    ViewMode,
};

static PRIMARY: StoryDef = StoryDef::new("Forms/Button", "Primary", |_| {
    rsx! { p { class: "story", "primary-body" } }
});
static DANGER: StoryDef = StoryDef::new("Forms/Button", "Danger", |_| {
    rsx! { p { class: "story", "danger-body" } }
});
static CARD: StoryDef = StoryDef::new("Layout/Card", "Default", |_| {
    rsx! { p { class: "story", "card-body" } }
});

static ALL: &[&StoryDef] = &[&PRIMARY, &DANGER, &CARD];

// ------------------------------------------------------- the two halves part

#[test]
fn the_manager_document_holds_no_story_only_a_frame_to_one() {
    let docs = Documents::new(ALL);
    let html = docs.manager_html();

    assert!(html.contains("<iframe"), "the manager must frame the preview: {html}");
    assert!(
        html.contains("viewMode=preview"),
        "the frame must point back at this app in the other role: {html}"
    );
    // The whole point of the milestone: no story markup in the shell's document.
    for body in ["primary-body", "danger-body", "card-body"] {
        assert!(!html.contains(body), "story markup leaked into the manager: {html}");
    }
    // ...but the shell still knows the whole index, because both halves are one
    // bundle. This is what keeps `ArgType` a `&'static` on both sides.
    assert!(html.contains("Primary") && html.contains("Danger") && html.contains("Card"));
}

#[test]
fn the_preview_document_holds_a_story_and_none_of_the_shell() {
    let docs = Documents::new(ALL);
    let html = docs.preview_html();

    assert!(html.contains("primary-body"), "got: {html}");
    for shell in ["dxsb-sidebar", "dxsb-toolbar", "dxsb-panel", "dxsb-status", "<iframe"] {
        assert!(!html.contains(shell), "the shell leaked into the preview: {html}");
    }
}

#[test]
fn the_frame_url_is_fixed_at_mount_so_live_state_cannot_reload_it() {
    let mut docs = Documents::new(WITH_ARGS);
    let before = frame_src(&docs.manager_html());

    // A story writing its own args back — the one path that genuinely changes
    // the *manager's* state from outside, which is what makes this test
    // non-vacuous. If `src` tracked live state, this is where it would move.
    says(
        &mut docs,
        ViewMode::Preview,
        Event::RequestArgsUpdate {
            id: "forms-demo--default".into(),
            args: ArgMap::new().with("label", ArgValue::Text("Rewritten".into())),
        },
    );

    assert!(docs.manager_html().contains("Rewritten"), "the manager ignored the write-back");
    assert_eq!(
        before,
        frame_src(&docs.manager_html()),
        "the iframe src changed, which reloads the preview and defeats the channel"
    );
}

fn frame_src(html: &str) -> String {
    let (_, after) = html.split_once("<iframe").expect("no iframe in the manager");
    let (_, after) = after.split_once("src=\"").expect("iframe has no src");
    after.split('"').next().unwrap().to_string()
}

// ----------------------------------------------------------- the conversation

#[test]
fn the_preview_announces_itself_and_the_manager_answers_with_the_current_state() {
    let docs = Documents::new(ALL);

    let from_preview = docs.bus.sent_by(ViewMode::Preview);
    assert!(
        from_preview.contains(&Event::PreviewReady),
        "the preview must announce itself; a manager that mounted first would \
         otherwise talk to an empty room: {from_preview:?}"
    );

    let from_manager = docs.bus.sent_by(ViewMode::Manager);
    assert!(
        from_manager.iter().any(|e| matches!(
            e,
            Event::SetCurrentStory { id } if id == "forms-button--primary"
        )),
        "the manager must (re)publish the current story on the handshake: {from_manager:?}"
    );
}

#[test]
fn selecting_a_story_in_the_manager_changes_what_the_preview_renders() {
    let mut docs = Documents::new(ALL);
    assert!(docs.preview_html().contains("primary-body"));

    select(&mut docs, "forms-button--danger");
    docs.settle();

    assert!(docs.preview_html().contains("danger-body"), "got: {}", docs.preview_html());
    assert!(!docs.preview_html().contains("primary-body"));
}

#[test]
fn a_story_default_reaches_the_controls_panel_only_by_crossing_the_wire() {
    // The manager cannot compute this value: `base_args` evaluates the story's
    // props, which needs a live scope, and the props live in the other document.
    // So a default showing up in the shell's markup is proof of the round trip.
    let docs = Documents::new(WITH_ARGS);
    assert!(
        docs.manager_html().contains("Ship it"),
        "the story's default never crossed: {}",
        docs.manager_html()
    );
}

#[test]
fn editing_a_control_in_the_manager_reaches_the_story_in_the_other_document() {
    let mut docs = Documents::new(WITH_ARGS);
    assert!(docs.preview_html().contains("Ship it"));

    set_arg(&mut docs, "label", ArgValue::Text("Edited".into()));
    docs.settle();

    assert!(docs.preview_html().contains("Edited"), "got: {}", docs.preview_html());
}

#[test]
fn a_stale_link_is_reported_back_from_the_document_that_could_not_find_it() {
    let mut docs = Documents::new(ALL);
    select(&mut docs, "forms-button--gone");
    docs.settle();

    assert!(
        docs.bus
            .sent_by(ViewMode::Preview)
            .iter()
            .any(|e| matches!(e, Event::StoryMissing { id } if id == "forms-button--gone")),
        "traffic was: {:?}",
        docs.bus.traffic.borrow()
    );
}

// --------------------------------------------------------- the load-bearing no

#[test]
fn neither_half_relies_on_hearing_its_own_messages() {
    // `InProcessChannel` broadcasts an emit to every listener, the emitter's
    // included. `postMessage` does not — it only reaches the peer. This harness
    // has the stricter behaviour, so the suite passing at all is the assertion.
    // What is checked here is that the traffic is genuinely two-sided, i.e. the
    // test above is not passing because nothing was ever sent.
    let docs = Documents::new(WITH_ARGS);
    assert!(!docs.bus.sent_by(ViewMode::Manager).is_empty());
    assert!(!docs.bus.sent_by(ViewMode::Preview).is_empty());
}

// ------------------------------------------------------------------- fixtures

/// A props type with a default worth watching cross the wire.
#[derive(Clone, PartialEq)]
struct DemoProps {
    label: String,
}

static DEMO_ARG_TYPES: &[dioxus_storybook_core::ArgType] = &[dioxus_storybook_core::ArgType {
    name: "label",
    ty: "String",
    docs: "Text shown inside.",
    control: dioxus_storybook_core::Control::Text,
    required: true,
}];

impl Controllable for DemoProps {
    fn arg_types() -> &'static [dioxus_storybook_core::ArgType] {
        DEMO_ARG_TYPES
    }
    fn apply(&self, args: &ArgMap) -> Self {
        Self { label: args.get_or("label", self.label.clone()) }
    }
    fn to_args(&self) -> ArgMap {
        ArgMap::new().with("label", ArgValue::Text(self.label.clone()))
    }
    fn wire_actions(&self, _sink: &dioxus_storybook_core::ActionSink) -> Self {
        self.clone()
    }
}

fn demo_base() -> DemoProps {
    DemoProps { label: "Ship it".into() }
}

static DEMO: StoryDef = StoryDef::new("Forms/Demo", "Default", |args| {
    let label = demo_base().apply(args).label;
    rsx! { p { class: "story", "{label}" } }
})
.with_arg_types(DemoProps::arg_types)
.with_base_args(|| demo_base().to_args());

static WITH_ARGS: &[&StoryDef] = &[&DEMO];

// ------------------------------------------------------------------- driving

/// Say something on the wire as one half, then let both settle.
///
/// The shell's controls are DOM events and a headless `VirtualDom` cannot
/// deliver one, so what a click *would have put on the wire* is put there
/// directly. The widgets themselves are covered by `tests/panels.rs`.
fn says(docs: &mut Documents, from: ViewMode, event: Event) {
    docs.bus.inject(from, event);
    docs.settle();
}

fn select(docs: &mut Documents, id: &str) {
    says(docs, ViewMode::Manager, Event::SetCurrentStory { id: id.to_string() });
}

fn set_arg(docs: &mut Documents, name: &str, value: ArgValue) {
    says(
        docs,
        ViewMode::Manager,
        Event::UpdateArgs {
            id: "forms-demo--default".into(),
            args: ArgMap::new().with(name, value),
        },
    );
}

// ------------------------------------------------------------ the frame dying

#[test]
fn a_panic_in_the_preview_is_reported_in_the_shell_with_a_way_back() {
    let mut docs = Documents::new(ALL);
    assert!(!docs.manager_html().contains("The preview stopped"));

    // What the preview's panic hook posts on its way out. It is the last thing
    // that document ever sends: `panic = "abort"` means the module is gone, not
    // unwound.
    says(
        &mut docs,
        ViewMode::Preview,
        Event::PreviewPanicked {
            message: "panicked at 'index out of bounds', src/stories/button.rs:31".into(),
        },
    );

    let html = docs.manager_html();
    assert!(html.contains("The preview stopped"), "no notice: {html}");
    assert!(html.contains("src/stories/button.rs:31"), "the message was dropped: {html}");
    assert!(
        html.contains("Reload the preview"),
        "a dead frame with no way back is a dead end: {html}"
    );
}

#[test]
fn a_preview_that_comes_back_clears_the_notice() {
    let mut docs = Documents::new(ALL);
    says(
        &mut docs,
        ViewMode::Preview,
        Event::PreviewPanicked { message: "boom".into() },
    );
    assert!(docs.manager_html().contains("The preview stopped"));

    // A reloaded frame introduces itself again, exactly as it did on first load.
    says(&mut docs, ViewMode::Preview, Event::PreviewReady);

    assert!(
        !docs.manager_html().contains("The preview stopped"),
        "the notice outlived the document it was about: {}",
        docs.manager_html()
    );
}

// ---------------------------------------------------- project-level wrapping

/// The reason project decorators matter more after M3 than before: the story
/// renders in a document that has none of the app's CSS in it, and a decorator
/// is what puts it there. This one stands in for a stylesheet link.
static PROJECT_DECORATORS: &[Decorator] = &[|_ctx: &StoryContext, story: Element| {
    rsx! {
        div { class: "project-shell",
            style { ".chip{{color:red}}" }
            {story}
        }
    }
}];

#[test]
fn project_decorators_wrap_stories_in_the_preview_and_not_in_the_shell() {
    let docs = Documents::with_project(
        ALL,
        dioxus_storybook_core::Project::new().with_decorators(PROJECT_DECORATORS),
    );

    let preview = docs.preview_html();
    assert!(preview.contains("project-shell"), "the decorator did not run: {preview}");
    assert!(
        preview.find("project-shell") < preview.find("primary-body"),
        "the decorator must wrap the story, not follow it: {preview}"
    );

    // The shell renders no stories, so it must not be running story decorators
    // either — that would be the first crack in the split.
    assert!(!docs.manager_html().contains("project-shell"));
}

// ------------------------------------------------------------------ globals

static GLOBALS: &[GlobalType] = &[
    GlobalType::select("theme", &["light", "dark"]).with_title("Theme"),
    GlobalType::toggle("rtl", false),
];

/// Reports the globals it was given, so what crossed the wire is visible in the
/// preview's markup.
static REPORTER: &[Decorator] = &[|ctx: &StoryContext, story: Element| {
    let theme = ctx.globals().get("theme").map(ArgValue::as_text).unwrap_or_default();
    let rtl = ctx.globals().get("rtl").map(ArgValue::as_text).unwrap_or_default();
    rsx! { div { class: "theme-{theme} rtl-{rtl}", {story} } }
}];

fn with_globals() -> dioxus_storybook_core::Project {
    dioxus_storybook_core::Project::new()
        .with_globals(GLOBALS)
        .with_decorators(REPORTER)
}

#[test]
fn the_toolbar_appears_in_the_shell_and_the_defaults_reach_the_story() {
    let docs = Documents::with_project(ALL, with_globals());

    let manager = docs.manager_html();
    // `class="..."`, not the bare class name: the manager inlines its whole
    // stylesheet, which mentions every class it styles. A bare `contains` here
    // passes whether the toolbar rendered or not.
    assert!(manager.contains(r#"class="dxsb-globals""#), "no toolbar: {manager}");
    assert!(manager.contains("Theme"), "the global's title is missing: {manager}");

    // The declarations are `&'static` and both halves share the bundle, so each
    // side fills in the defaults itself — nothing had to be sent for this.
    assert!(
        docs.preview_html().contains(r#"class="theme-light rtl-false""#),
        "got: {}",
        docs.preview_html()
    );
}

#[test]
fn changing_a_global_reaches_the_story_in_the_other_document() {
    let mut docs = Documents::with_project(ALL, with_globals());
    says(
        &mut docs,
        ViewMode::Manager,
        Event::SetGlobals {
            globals: ArgMap::new()
                .with("theme", ArgValue::Variant("dark".into()))
                .with("rtl", ArgValue::Bool(true)),
        },
    );
    assert!(
        docs.preview_html().contains(r#"class="theme-dark rtl-true""#),
        "got: {}",
        docs.preview_html()
    );
}

#[test]
fn a_global_outlives_the_story_it_was_set_on() {
    // The whole reason globals are not args: flip the theme, then walk the
    // sidebar looking for the component that forgot about it.
    let mut docs = Documents::with_project(ALL, with_globals());
    says(
        &mut docs,
        ViewMode::Manager,
        Event::SetGlobals {
            globals: ArgMap::new().with("theme", ArgValue::Variant("dark".into())),
        },
    );
    select(&mut docs, "layout-card--default");

    let preview = docs.preview_html();
    assert!(preview.contains("card-body"), "the story did not change: {preview}");
    assert!(preview.contains("theme-dark"), "the global did not survive: {preview}");
}

#[test]
fn only_the_selections_travel_never_the_resolved_set() {
    // Both halves read the same `&'static` declarations, so sending the defaults
    // would only make every message longer and give the two sides a chance to
    // disagree about what the defaults are.
    let mut docs = Documents::with_project(ALL, with_globals());
    docs.bus.traffic.borrow_mut().clear();
    says(
        &mut docs,
        ViewMode::Manager,
        Event::SetGlobals {
            globals: ArgMap::new().with("theme", ArgValue::Variant("dark".into())),
        },
    );

    let sent: Vec<_> = docs
        .bus
        .sent_by(ViewMode::Manager)
        .into_iter()
        .filter_map(|e| match e {
            Event::SetGlobals { globals } => Some(globals),
            _ => None,
        })
        .collect();
    assert!(!sent.is_empty(), "nothing was sent");
    for globals in sent {
        assert!(
            globals.get("rtl").is_none(),
            "an untouched global rode along: {globals:?}"
        );
    }
}

#[test]
fn the_shell_shows_no_toolbar_when_the_project_declares_no_globals() {
    // An empty bar would be a row of nothing with a border around it.
    //
    // Matched as `class="..."` for the same reason as above — the stylesheet is
    // inlined into this document and names the class regardless.
    let docs = Documents::new(ALL);
    assert!(!docs.manager_html().contains(r#"class="dxsb-globals""#));
}

// ------------------------------------------------------------------- layout

static FULLSCREEN: StoryDef = StoryDef::new("Layout/Full", "Default", |_| {
    rsx! { p { "full-body" } }
})
.with_parameters(dioxus_storybook_core::Parameters::from_static(&[(
    "layout",
    dioxus_storybook_core::ParamValue::Str("fullscreen"),
)]));

static TYPO: StoryDef = StoryDef::new("Layout/Typo", "Default", |_| rsx! { p { "typo-body" } })
    .with_parameters(dioxus_storybook_core::Parameters::from_static(&[(
        "layout",
        dioxus_storybook_core::ParamValue::Str("fulscreen"),
    )]));

static LAYOUTS: &[&StoryDef] = &[&FULLSCREEN, &TYPO];

#[test]
fn a_story_can_ask_the_canvas_not_to_frame_it() {
    let docs = Documents::new(LAYOUTS);
    assert!(
        docs.preview_html().contains(r#"class="dxsb-canvas layout-fullscreen""#),
        "got: {}",
        docs.preview_html()
    );
}

#[test]
fn a_layout_this_build_does_not_understand_still_renders() {
    // Parameters are an open, stringly-typed space shared with addons that may
    // not be in this build. A typo — or a value from a newer addon — must not
    // be the difference between seeing a story and seeing nothing.
    let mut docs = Documents::new(LAYOUTS);
    select(&mut docs, "layout-typo--default");

    let preview = docs.preview_html();
    assert!(preview.contains("typo-body"), "got: {preview}");
    assert!(preview.contains(r#"class="dxsb-canvas layout-centered""#), "got: {preview}");
}

#[test]
fn the_shell_publishes_globals_even_with_no_story_selected() {
    // A global is not about any one story, and an empty book still has a theme.
    // This is also what makes the reset button honest: an empty selection map
    // means "nothing has been changed", and it can only mean that if a default
    // never gets written into it.
    let docs = Documents::with_project(&[], with_globals());
    assert!(
        docs.bus
            .sent_by(ViewMode::Manager)
            .iter()
            .any(|e| matches!(e, Event::SetGlobals { globals } if globals.is_empty())),
        "traffic was: {:?}",
        docs.bus.traffic.borrow()
    );
}

// ----------------------------------------------------------------- viewport

static SIZES: &[dioxus_storybook_core::Viewport] = &[
    dioxus_storybook_core::Viewport::new("phone", "Phone", 390, 844),
    dioxus_storybook_core::Viewport::new("desk", "Desk", 1440, 900),
];

/// Asks for a phone the way a mobile nav drawer would: statically, per story.
static ON_A_PHONE: StoryDef = StoryDef::new("Layout/Drawer", "Default", |_| {
    rsx! { p { "drawer-body" } }
})
.with_parameters(dioxus_storybook_core::Parameters::from_static(&[(
    "viewport",
    dioxus_storybook_core::ParamValue::Str("phone"),
)]));

static RESPONSIVE_STORY: StoryDef =
    StoryDef::new("Layout/Page", "Default", |_| rsx! { p { "page-body" } });

static SIZED: &[&StoryDef] = &[&ON_A_PHONE, &RESPONSIVE_STORY];
/// The same two, the other way round, so the shell *lands* on the story that
/// asks for nothing. Which story the manager is showing is decided by its own
/// signal, and the manager never hears its own `SetCurrentStory`.
static UNSIZED_FIRST: &[&StoryDef] = &[&RESPONSIVE_STORY, &ON_A_PHONE];

/// Reports the size the preview believes it is, so the *frame's* width and the
/// *story's* idea of it can be compared across the wire.
static VIEWPORT_REPORTER: &[Decorator] = &[|ctx: &StoryContext, story: Element| {
    let seen = match ctx.viewport() {
        Some(view) => format!("{}-{}x{}", view.name(), view.width(), view.height()),
        None => "responsive".to_string(),
    };
    rsx! { div { class: "vp-{seen}", {story} } }
}];

fn with_viewports() -> dioxus_storybook_core::Project {
    dioxus_storybook_core::Project::new()
        .with_viewports(SIZES)
        .with_decorators(VIEWPORT_REPORTER)
}

#[test]
fn a_story_can_ask_to_open_at_a_size_and_both_halves_agree_on_it() {
    // The addon in one assertion: the shell sizes the frame, and the story —
    // in the other document, over the wire — is told the same numbers. They
    // agree because both halves call `viewport::resolve`, not because anything
    // was synchronised.
    let docs = Documents::with_project(SIZED, with_viewports());

    let manager = docs.manager_html();
    assert!(
        manager.contains(r#"class="dxsb-stage sized""#),
        "the frame was not sized: {manager}"
    );
    assert!(
        manager.contains("width:390px;height:844px"),
        "the frame is the wrong size: {manager}"
    );
    assert!(
        docs.preview_html().contains(r#"class="vp-phone-390x844""#),
        "the story was not told its size: {}",
        docs.preview_html()
    );
}

#[test]
fn a_responsive_canvas_puts_no_size_on_the_frame() {
    let docs = Documents::with_project(UNSIZED_FIRST, with_viewports());

    let manager = docs.manager_html();
    // `class="..."`, not a bare class name: this document inlines the whole
    // stylesheet, which names `.dxsb-stage.sized` whether one rendered or not.
    assert!(!manager.contains(r#"class="dxsb-stage sized""#), "got: {manager}");
    assert!(manager.contains(r#"class="dxsb-stage""#), "got: {manager}");
    assert!(
        docs.preview_html().contains(r#"class="vp-responsive""#),
        "got: {}",
        docs.preview_html()
    );
}

#[test]
fn the_viewport_travels_as_a_global_and_outlives_the_story_it_was_set_on() {
    // It is carried in the globals map rather than a channel of its own, which
    // is what gives it a URL encoding, a wire encoding and a place in
    // `StoryContext` with no new code. This is the wire half of that claim.
    let mut docs = Documents::with_project(SIZED, with_viewports());
    says(
        &mut docs,
        ViewMode::Manager,
        Event::SetGlobals {
            globals: ArgMap::new()
                .with("viewport", ArgValue::Variant("desk".into()))
                .with("viewport-rotated", ArgValue::Bool(true)),
        },
    );
    assert!(
        docs.preview_html().contains(r#"class="vp-desk-900x1440""#),
        "got: {}",
        docs.preview_html()
    );

    // ...and a chosen size is not a property of the story you chose it on.
    select(&mut docs, "layout-page--default");
    let preview = docs.preview_html();
    assert!(preview.contains("page-body"), "the story did not change: {preview}");
    assert!(preview.contains("vp-desk-900x1440"), "the viewport did not survive: {preview}");
}

#[test]
fn a_selection_beats_the_story_parameter_across_the_wire() {
    let mut docs = Documents::with_project(SIZED, with_viewports());
    assert!(docs.preview_html().contains("vp-phone-390x844"));
    says(
        &mut docs,
        ViewMode::Manager,
        Event::SetGlobals {
            globals: ArgMap::new().with("viewport", ArgValue::Variant("responsive".into())),
        },
    );
    assert!(
        docs.preview_html().contains(r#"class="vp-responsive""#),
        "a phone story could not be looked at full width: {}",
        docs.preview_html()
    );
}

#[test]
fn the_picker_is_there_by_default_and_gone_when_the_project_offers_no_sizes() {
    let docs = Documents::with_project(SIZED, with_viewports());
    assert!(
        docs.manager_html().contains(r#"class="dxsb-viewport""#),
        "no picker: {}",
        docs.manager_html()
    );

    let none = Documents::with_project(
        SIZED,
        dioxus_storybook_core::Project::new()
            .with_viewports(&[])
            .with_decorators(VIEWPORT_REPORTER),
    );
    let manager = none.manager_html();
    assert!(!manager.contains(r#"class="dxsb-viewport""#), "got: {manager}");
    // ...and the story that asked for a phone gets a responsive canvas, rather
    // than a size nothing in this build can offer.
    assert!(!manager.contains(r#"class="dxsb-stage sized""#), "got: {manager}");
    assert!(none.preview_html().contains(r#"class="vp-responsive""#));
}
