//! URL state is what carries the dev loop across a rebuild, so it has to be
//! total: every malformed input must land on *something*, never panic.

use dioxus_storybook_core::{ArgMap, ArgValue, UrlState, url};

#[test]
fn round_trips_id_and_args() {
    let state = UrlState {
        id: Some("forms-button--primary".into()),
        args: ArgMap::new()
            .with("label", ArgValue::Text("Hey there".into()))
            .with("disabled", ArgValue::Bool(true))
            .with("scale", ArgValue::Num(1.5)),
    };
    let query = state.to_query();
    assert_eq!(
        query,
        "?id=forms-button--primary&args=disabled:!true;label:Hey%20there;scale:1.5"
    );
    let parsed = UrlState::parse(&query);
    assert_eq!(parsed.id, state.id);
    assert_eq!(parsed.args.get("disabled"), Some(&ArgValue::Bool(true)));
    assert_eq!(
        parsed.args.get("label"),
        Some(&ArgValue::Text("Hey there".into()))
    );
    assert_eq!(parsed.args.get("scale"), Some(&ArgValue::Num(1.5)));
}

#[test]
fn whole_numbers_lose_their_trailing_zero() {
    let args = ArgMap::new().with("scale", ArgValue::Num(1.0));
    assert_eq!(url::encode_args(&args), "scale:1");
}

#[test]
fn null_and_booleans_use_reserved_literals() {
    let args = ArgMap::new()
        .with("a", ArgValue::Bool(false))
        .with("b", ArgValue::Null);
    assert_eq!(url::encode_args(&args), "a:!false;b:!null");
    let back = url::decode_args("a:!false;b:!null");
    assert_eq!(back.get("a"), Some(&ArgValue::Bool(false)));
    assert_eq!(back.get("b"), Some(&ArgValue::Null));
}

#[test]
fn text_that_looks_like_a_literal_is_escaped_not_confused() {
    let args = ArgMap::new().with("label", ArgValue::Text("!true".into()));
    let encoded = url::encode_args(&args);
    assert_eq!(encoded, "label:%21true");
    assert_eq!(
        url::decode_args(&encoded).get("label"),
        Some(&ArgValue::Text("!true".into())),
        "escaping keeps user text out of the reserved namespace"
    );
}

#[test]
fn separators_inside_values_survive() {
    let args = ArgMap::new().with("q", ArgValue::Text("a;b:c&d=e%f".into()));
    let encoded = url::encode_args(&args);
    let back = url::decode_args(&encoded);
    assert_eq!(back.get("q"), Some(&ArgValue::Text("a;b:c&d=e%f".into())));
}

#[test]
fn variants_encode_as_plain_text_and_the_applier_retypes_them() {
    // The URL is deliberately untyped: `Variant` and `Text` are the same on the
    // wire. Typing is restored by the generated applier, which knows the field
    // is an enum. See the args module docs.
    let args = ArgMap::new().with("variant", ArgValue::Variant("Danger".into()));
    let back = url::decode_args(&url::encode_args(&args));
    assert_eq!(back.get("variant"), Some(&ArgValue::Text("Danger".into())));
}

#[test]
fn malformed_input_degrades_instead_of_panicking() {
    for junk in [
        "",
        "?",
        "??&&==",
        "id=",
        "args=",
        "args=nocolon",
        "args=:novalue",
        "args=k:%",
        "args=k:%ZZ",
        "args=;;;;",
        "id=a&id=b",
        "unknown=1",
    ] {
        let state = UrlState::parse(junk);
        // The only contract: it returns.
        let _ = state.to_query();
    }
    assert_eq!(UrlState::parse("id=").id, None);
    assert_eq!(UrlState::parse("id=a&id=b").id, Some("b".into()));
    assert!(UrlState::parse("args=nocolon").args.is_empty());
}

#[test]
fn an_empty_state_still_produces_a_usable_query() {
    assert_eq!(UrlState::default().to_query(), "?");
}
