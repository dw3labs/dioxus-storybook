//! The `postMessage` codec.
//!
//! From M3 an `Event` crosses a document boundary as a string, so these are the
//! tests that stop a message from arriving as something other than what was
//! sent. The round trip is asserted for every variant — deliberately by
//! enumeration rather than a loop, so adding a variant without a test is a
//! visible omission rather than an invisible one.

use dioxus_storybook_core::wire::{MAGIC, ViewMode, decode, encode};
use dioxus_storybook_core::{ArgMap, ArgValue, Event};

const M: ViewMode = ViewMode::Manager;
const P: ViewMode = ViewMode::Preview;

/// Send as `from`; read it back in the document on the other end.
fn round_trip(from: ViewMode, event: Event) {
    let raw = encode(from, &event);
    assert_eq!(decode(from.peer(), &raw), Some(event.clone()), "raw was: {raw}");
}

fn args() -> ArgMap {
    ArgMap::new()
        .with("label", ArgValue::Text("Hey there".into()))
        .with("disabled", ArgValue::Bool(true))
        .with("scale", ArgValue::Num(1.5))
        .with(
            "classes",
            ArgValue::List(vec![ArgValue::Text("a".into()), ArgValue::Text("b".into())]),
        )
}

#[test]
fn every_event_survives_the_round_trip() {
    let id = || "forms-button--primary".to_string();
    round_trip(P, Event::PreviewReady);
    round_trip(M, Event::SetCurrentStory { id: id() });
    round_trip(M, Event::UpdateArgs { id: id(), args: args() });
    round_trip(P, Event::RequestArgsUpdate { id: id(), args: args() });
    round_trip(P, Event::StoryPrepared { id: id(), initial_args: args() });
    round_trip(M, Event::SetGlobals { globals: args() });
    round_trip(M, Event::ResetArgs { id: id() });
    round_trip(P, Event::StoryRendered { id: id() });
    round_trip(P, Event::StoryMissing { id: id() });
    round_trip(
        P,
        Event::ActionLogged { name: "onclick".into(), payload: "MouseEvent { .. }".into() },
    );
    round_trip(
        P,
        Event::PreviewPanicked { message: "index out of bounds at story.rs:12".into() },
    );
}

#[test]
fn a_payload_may_contain_the_delimiters() {
    // An action payload is arbitrary `Debug` output. If the codec split on a
    // character instead of counting bytes, this is the message that would
    // arrive mangled — or not at all.
    let event = Event::ActionLogged {
        name: "onchange".into(),
        payload: "Text { raw: \"a|b:c|9:x\", lines: [\"a\", \"b\"] }\nnext".into(),
    };
    round_trip(P, event);
}

#[test]
fn a_payload_may_be_empty_or_multibyte() {
    round_trip(P, Event::ActionLogged { name: "onclick".into(), payload: String::new() });
    round_trip(
        P,
        Event::ActionLogged { name: "onclick".into(), payload: "héllo — 世界 🎛".into() },
    );
}

#[test]
fn a_listener_ignores_its_own_voice() {
    // `window.parent` is `window` when a document is not framed, so a preview
    // opened directly would hear everything it says. The sender is on the wire
    // and a listener only accepts its peer.
    let raw = encode(P, &Event::StoryRendered { id: "x--y".into() });
    assert_eq!(decode(M, &raw), Some(Event::StoryRendered { id: "x--y".into() }));
    assert_eq!(decode(P, &raw), None, "a preview must not accept a preview's message");
    // ...and the same the other way round.
    let raw = encode(M, &Event::ResetArgs { id: "x--y".into() });
    assert_eq!(decode(P, &raw), Some(Event::ResetArgs { id: "x--y".into() }));
    assert_eq!(decode(M, &raw), None, "a manager must not accept a manager's message");
}

#[test]
fn foreign_traffic_on_the_message_bus_is_dropped() {
    // `window.onmessage` is shared with extensions, embeds and dev tooling.
    for foreign in [
        "",
        "hello",
        "{\"source\":\"react-devtools-bridge\"}",
        "dxsb0|manager|story|3:abc",
        "dxsb1",
        "dxsb1|",
        "dxsb1|manager",
    ] {
        assert_eq!(decode(P, foreign), None, "accepted foreign message: {foreign:?}");
        assert_eq!(decode(M, foreign), None, "accepted foreign message: {foreign:?}");
    }
}

#[test]
fn a_corrupt_length_prefix_is_rejected_rather_than_panicking() {
    for corrupt in [
        "dxsb1|manager|story|99:abc",  // longer than what follows
        "dxsb1|manager|story|abc",     // no length at all
        "dxsb1|manager|story|-1:abc",  // not a usize
        "dxsb1|manager|story|3abc",    // no colon
        "dxsb1|manager|story|1:é",     // length splits a UTF-8 sequence
        "dxsb1|manager|story",         // variant needs a field and has none
        "dxsb1|manager|args|3:abc",    // variant needs two fields and has one
    ] {
        assert_eq!(decode(P, corrupt), None, "accepted corrupt message: {corrupt:?}");
    }
}

#[test]
fn an_unknown_tag_from_a_newer_peer_is_dropped_not_fatal() {
    // A stale cached iframe against a fresh manager, or the reverse.
    //
    // The tag here is deliberately one that will never be real. An earlier
    // version of this test used "globals", which stopped being unknown one
    // session later — a test whose subject can quietly become its opposite.
    assert_eq!(decode(P, "dxsb1|manager|no-such-tag-will-ever-exist|3:abc"), None);
}

#[test]
fn every_message_is_recognisable_as_ours_before_it_is_parsed() {
    let raw = encode(M, &Event::SetCurrentStory { id: "a--b".into() });
    assert!(raw.starts_with(MAGIC), "got: {raw}");
    assert!(raw.starts_with("dxsb1|manager|"), "got: {raw}");
}

#[test]
fn the_view_mode_of_a_bare_url_is_the_manager() {
    assert_eq!(ViewMode::parse("preview"), ViewMode::Preview);
    assert_eq!(ViewMode::parse("manager"), ViewMode::Manager);
    assert_eq!(ViewMode::parse(""), ViewMode::Manager);
    assert_eq!(ViewMode::parse("nonsense"), ViewMode::Manager);
    assert_eq!(ViewMode::Manager.peer(), ViewMode::Preview);
    assert_eq!(ViewMode::Preview.peer(), ViewMode::Manager);
}
