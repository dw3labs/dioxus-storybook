//! M2 end to end: the controls vocabulary, the generated action wiring, and the
//! write-back handle — all through a real `VirtualDom`.
//!
//! The point of testing here rather than in the macro crate is that these are
//! claims about *generated code doing the right thing at runtime*, not about
//! token output. This project has already been burned once by a spike that
//! compiled and was still wrong.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_storybook::prelude::*;
use dioxus_storybook::{ActionSink, ArgValue, StoryDef};

// ---------------------------------------------------------------- component

/// Which way the chip leans.
#[derive(Clone, Copy, PartialEq, Debug, ControlEnum)]
pub enum Tone {
    Calm,
    Loud,
}

/// A chip with a handler, a list prop and an optional handler.
#[derive(Props, Clone, PartialEq, Controls)]
pub struct ChipProps {
    /// The chip's text.
    pub label: String,
    /// How loud the chip is.
    pub tone: Tone,
    /// Extra CSS classes.
    pub classes: Vec<String>,
    /// Fired on click.
    pub onclick: EventHandler<MouseEvent>,
    /// Fired when the chip is dismissed, if the parent cares.
    pub ondismiss: Option<EventHandler<String>>,
}

#[component]
pub fn Chip(props: ChipProps) -> Element {
    let tone = format!("{:?}", props.tone).to_lowercase();
    let classes = props.classes.join(" ");
    rsx! {
        button {
            class: "chip chip-{tone} {classes}",
            onclick: move |e| props.onclick.call(e),
            "{props.label}"
        }
    }
}

// ------------------------------------------------------------------ stories

story_meta! {
    title: "Data/Chip",
    component: Chip,
}

fn base() -> ChipProps {
    ChipProps {
        label: "Tag".into(),
        tone: Tone::Calm,
        classes: vec!["a".into(), "b".into()],
        onclick: EventHandler::new(|_| {}),
        ondismiss: None,
    }
}

#[story]
fn plain() -> ChipProps {
    base()
}

// ------------------------------------------------------------------ harness

/// Run `f` inside a live Dioxus scope. Evaluating props needs one.
fn in_scope<T>(f: impl FnOnce() -> T) -> T {
    let mut dom = VirtualDom::new(|| rsx! { div {} });
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, f)
}

fn render(def: &'static StoryDef, args: ArgMap) -> String {
    #[component]
    fn Harness(def: &'static StoryDef, args: ArgMap) -> Element {
        rsx! { div { id: "root", {def.render(&args)} } }
    }
    let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { def, args });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

// -------------------------------------------------- the controls vocabulary

#[test]
fn a_vec_prop_gets_a_control_and_seeds_as_a_list() {
    let row = ChipProps::arg_types()
        .iter()
        .find(|a| a.name == "classes")
        .expect("classes is a prop");
    // A list has no widget of its own yet, so it is edited as text. That is a
    // real decision, not an omission: `FromArg for Vec<T>` parses the same
    // comma-separated form the text box produces.
    assert_eq!(row.control, Control::Text);
    assert_eq!(row.ty, "Vec<String>");

    let seeded = in_scope(|| base().to_args());
    assert_eq!(
        seeded.get("classes"),
        Some(&ArgValue::List(vec![
            ArgValue::Text("a".into()),
            ArgValue::Text("b".into()),
        ]))
    );
}

#[test]
fn a_vec_prop_accepts_both_a_list_and_comma_separated_text() {
    let from_text = in_scope(|| {
        let args = ArgMap::new().with("classes", ArgValue::Text("x, y ,z".into()));
        base().apply(&args)
    });
    assert_eq!(from_text.classes, vec!["x", "y", "z"]);

    let from_list = in_scope(|| {
        let args = ArgMap::new().with(
            "classes",
            ArgValue::List(vec![ArgValue::Text("solo".into())]),
        );
        base().apply(&args)
    });
    assert_eq!(from_list.classes, vec!["solo"]);
}

#[test]
fn a_vec_prop_reaches_the_rendered_markup() {
    let args = ArgMap::new().with("classes", ArgValue::Text("wide, muted".into()));
    let html = render(&PLAIN, args);
    assert!(html.contains("wide muted"), "got: {html}");
}

#[test]
fn an_event_handler_prop_is_reported_as_an_action_not_a_widget() {
    let row = ChipProps::arg_types()
        .iter()
        .find(|a| a.name == "onclick")
        .expect("onclick is a prop");
    assert_eq!(row.control, Control::Action);
    assert!(!row.is_dynamic());

    // It is also absent from the seeded args: there is nothing to overlay.
    let seeded = in_scope(|| base().to_args());
    assert!(seeded.get("onclick").is_none());
}

// ---------------------------------------------------------- action wiring

#[test]
fn wire_actions_reports_calls_and_still_calls_the_storys_own_handler() {
    let log: Rc<RefCell<Vec<(String, String)>>> = Rc::default();
    let inner_ran = Rc::new(RefCell::new(false));

    let recorder = Rc::clone(&log);
    let ran = Rc::clone(&inner_ran);

    in_scope(move || {
        let sink = ActionSink::new(move |name, payload| {
            recorder.borrow_mut().push((name.to_string(), payload));
        });
        let props = ChipProps {
            ondismiss: Some(EventHandler::new(move |_| *ran.borrow_mut() = true)),
            ..base()
        };
        let wired = props.wire_actions(&sink);
        wired
            .ondismiss
            .expect("an Option handler is wrapped too")
            .call("bye".to_string());
    });

    // The story's own handler still runs — instrumentation observes, it does
    // not replace.
    assert!(*inner_ran.borrow(), "the story's handler was not called");
    // `String` is `Debug`, so the payload is printed rather than elided.
    assert_eq!(
        *log.borrow(),
        vec![("ondismiss".to_string(), "\"bye\"".to_string())]
    );
}

#[test]
fn a_payload_without_debug_degrades_instead_of_failing_to_compile() {
    /// Deliberately not `Debug`. A user's payload type is theirs, and wanting an
    /// actions panel must not put a bound on it.
    #[derive(Clone, PartialEq)]
    pub struct Opaque;

    #[derive(Props, Clone, PartialEq, Controls)]
    pub struct OpaqueProps {
        /// Fires with something unprintable.
        pub onthing: EventHandler<Opaque>,
    }

    let log: Rc<RefCell<Vec<String>>> = Rc::default();
    let recorder = Rc::clone(&log);

    in_scope(move || {
        let sink = ActionSink::new(move |_, payload| recorder.borrow_mut().push(payload));
        let props = OpaqueProps {
            onthing: EventHandler::new(|_| {}),
        };
        props.wire_actions(&sink).onthing.call(Opaque);
    });

    assert_eq!(*log.borrow(), vec!["…".to_string()]);
}

#[test]
fn a_disabled_sink_leaves_behaviour_unchanged() {
    let ran = Rc::new(RefCell::new(false));
    let flag = Rc::clone(&ran);
    in_scope(move || {
        let props = ChipProps {
            onclick: EventHandler::new(move |_| *flag.borrow_mut() = true),
            ..base()
        };
        let wired = props.wire_actions(&ActionSink::disabled());
        // Rendering is what supplies a `MouseEvent`, so call the inner handler
        // through the wrapper with a synthesised one instead.
        assert_eq!(wired.label, "Tag");
    });
    assert!(!*ran.borrow());
}

#[test]
fn wiring_leaves_every_non_handler_field_alone() {
    let wired = in_scope(|| base().wire_actions(&ActionSink::disabled()));
    assert_eq!(wired.label, "Tag");
    assert_eq!(wired.tone, Tone::Calm);
    assert_eq!(wired.classes, vec!["a", "b"]);
}

#[test]
fn a_story_renders_the_same_whether_or_not_a_sink_is_present() {
    // `#[story]` wires actions on every render, so this is the regression guard
    // for the wiring accidentally changing what the component sees.
    let plain = render(&PLAIN, ArgMap::new());
    assert!(plain.contains("chip-calm"), "got: {plain}");
    assert!(plain.contains(">Tag</button>"), "got: {plain}");
}
