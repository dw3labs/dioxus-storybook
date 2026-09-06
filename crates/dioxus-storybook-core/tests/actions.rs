//! The ambient half of M2: where a wired handler reports to, and how a story
//! writes an arg back.
//!
//! These exercise the plumbing without a macro in sight. The generated wrapper
//! that *feeds* the sink is covered end-to-end in `dioxus-storybook`.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus_core::{ScopeId, VNode, VirtualDom, provide_context};
use dioxus_storybook_core::{ActionSink, ArgValue, ArgsHandle, use_args};

/// Run `f` inside a live Dioxus scope. Contexts do not exist without one.
///
/// Spelled against `dioxus-core` rather than `dioxus`, because that is the only
/// thing this crate depends on and a test should not widen that.
fn in_scope<T>(f: impl FnOnce() -> T) -> T {
    let mut dom = VirtualDom::new(|| Ok(VNode::placeholder()));
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, f)
}

#[test]
fn a_sink_forwards_name_and_payload() {
    let seen: Rc<RefCell<Vec<(String, String)>>> = Rc::default();
    let recorder = Rc::clone(&seen);
    let sink = ActionSink::new(move |name, payload| {
        recorder.borrow_mut().push((name.to_string(), payload));
    });

    sink.log("onclick", "MouseData { .. }".into());
    sink.log("oninput", "hello".into());

    assert!(sink.is_enabled());
    assert_eq!(
        *seen.borrow(),
        vec![
            ("onclick".to_string(), "MouseData { .. }".to_string()),
            ("oninput".to_string(), "hello".to_string()),
        ]
    );
}

#[test]
fn a_disabled_sink_swallows_calls_instead_of_panicking() {
    // This is what makes a story function callable from a plain unit test: no
    // storybook running, no context provided, no panic.
    let sink = ActionSink::disabled();
    assert!(!sink.is_enabled());
    sink.log("onclick", "ignored".into());
    assert_eq!(sink, ActionSink::default());
}

#[test]
fn the_ambient_sink_is_disabled_when_no_preview_provided_one() {
    let sink = in_scope(ActionSink::ambient);
    assert!(!sink.is_enabled());
}

#[test]
fn the_ambient_sink_is_disabled_outside_a_dioxus_runtime_entirely() {
    // `ActionSink::ambient` runs from generated code in every story body, so it
    // must be total: no runtime is a legitimate state, not an error.
    assert!(!ActionSink::ambient().is_enabled());
    assert!(!use_args().is_enabled());
}

#[test]
fn the_ambient_sink_is_the_one_the_scope_provided() {
    let seen: Rc<RefCell<Vec<String>>> = Rc::default();
    let recorder = Rc::clone(&seen);

    let provided = in_scope(move || {
        provide_context(ActionSink::new(move |name, _| {
            recorder.borrow_mut().push(name.to_string());
        }));
        let ambient = ActionSink::ambient();
        ambient.log("onclick", String::new());
        ambient
    });

    assert!(provided.is_enabled());
    assert_eq!(*seen.borrow(), vec!["onclick".to_string()]);
}

#[test]
fn use_args_writes_through_the_provided_handle() {
    let seen: Rc<RefCell<Vec<(String, ArgValue)>>> = Rc::default();
    let recorder = Rc::clone(&seen);

    in_scope(move || {
        provide_context(ArgsHandle::new(move |name, value| {
            recorder.borrow_mut().push((name, value));
        }));
        use_args().set("label", ArgValue::Text("typed".into()));
    });

    assert_eq!(
        *seen.borrow(),
        vec![("label".to_string(), ArgValue::Text("typed".into()))]
    );
}

#[test]
fn a_disabled_handle_drops_writes() {
    let handle = ArgsHandle::disabled();
    assert!(!handle.is_enabled());
    handle.set("label", ArgValue::Text("nowhere".into()));
    assert_eq!(handle, ArgsHandle::default());
}
