//! URL state: which story is selected, and what its args are.
//!
//! This is load-bearing, not a sharing nicety. `dx serve` cannot hot-patch on
//! wasm, so every source edit triggers a full page reload, and a reload
//! destroys every in-memory signal — selected story, edited args, scroll. The
//! URL is what carries the dev loop across a rebuild. (M0 finding; this is why
//! the feature moved from M2 into M1.)
//!
//! # Format
//!
//! ```text
//! ?id=forms-button--primary&args=label:Hey;disabled:!true&globals=theme:dark
//! ```
//!
//! `args` are per-story overrides; `globals` are the toolbar's, and outlive the
//! story you were looking at when you set them. Both use the same encoding.
//!
//! Values are percent-encoded, with three literals reserved: `!true`, `!false`
//! and `!null`, and one prefix: `!,` opens a comma-separated list. Everything
//! else decodes as a number if it parses as one and as text otherwise.
//!
//! An empty list and a list holding one empty string both encode as `!,` and
//! both decode as the empty list. That is the only place the codec is lossy,
//! and it is the cheaper end of the trade against a heavier syntax.
//!
//! The encoding is deliberately untyped. A field declared `String` whose value
//! is `"42"` comes back as [`ArgValue::Num`], and that is fine: typing is
//! restored by the generated applier, which knows the real field types and
//! falls back to the story's default for anything unconvertible.

use crate::args::format_num;
use crate::{ArgMap, ArgValue};

/// Everything the URL carries about the current view.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UrlState {
    /// The selected story id, if the URL names one.
    pub id: Option<String>,
    /// Arg overrides layered on top of the story's defaults.
    pub args: ArgMap,
    /// Toolbar globals that have been moved off their declared defaults.
    ///
    /// Only the selections travel, not the resolved set: the declarations are
    /// `&'static` and both halves of the app read them directly, so putting the
    /// defaults in the link would only make it longer and stale.
    pub globals: ArgMap,
}

impl UrlState {
    /// Parse a query string, with or without a leading `?`.
    ///
    /// Unknown parameters are ignored, and a malformed `args` value yields an
    /// empty arg map rather than an error — a bad link should land on the
    /// story, not on a crash.
    pub fn parse(query: &str) -> Self {
        let query = query.strip_prefix('?').unwrap_or(query);
        let mut state = UrlState::default();
        for pair in query.split('&').filter(|p| !p.is_empty()) {
            let (key, value) = match pair.split_once('=') {
                Some(kv) => kv,
                None => (pair, ""),
            };
            match key {
                "id" => {
                    let id = percent_decode(value);
                    if !id.is_empty() {
                        state.id = Some(id);
                    }
                }
                "args" => state.args = decode_args(value),
                "globals" => state.globals = decode_args(value),
                _ => {}
            }
        }
        state
    }

    /// Render back to a query string, including the leading `?`.
    ///
    /// Returns `"?"` when there is nothing to encode, which is still a valid
    /// relative URL for `history.replaceState`.
    pub fn to_query(&self) -> String {
        let mut parts = Vec::new();
        if let Some(id) = &self.id {
            parts.push(format!("id={}", percent_encode(id)));
        }
        if !self.args.is_empty() {
            parts.push(format!("args={}", encode_args(&self.args)));
        }
        if !self.globals.is_empty() {
            parts.push(format!("globals={}", encode_args(&self.globals)));
        }
        format!("?{}", parts.join("&"))
    }
}

/// Encode an arg map as `key:value;key:value`.
///
/// ```
/// # use dioxus_storybook_core::{ArgMap, ArgValue, url::encode_args};
/// let args = ArgMap::new()
///     .with("label", ArgValue::Text("Hey there".into()))
///     .with("disabled", ArgValue::Bool(true));
/// assert_eq!(encode_args(&args), "disabled:!true;label:Hey%20there");
/// ```
pub fn encode_args(args: &ArgMap) -> String {
    args.iter()
        .map(|(k, v)| format!("{}:{}", percent_encode(k), encode_value(v)))
        .collect::<Vec<_>>()
        .join(";")
}

/// Decode `key:value;key:value` back into an arg map.
///
/// Segments without a `:` are skipped.
pub fn decode_args(encoded: &str) -> ArgMap {
    let mut args = ArgMap::new();
    for segment in encoded.split(';').filter(|s| !s.is_empty()) {
        let Some((key, value)) = segment.split_once(':') else {
            continue;
        };
        let key = percent_decode(key);
        if key.is_empty() {
            continue;
        }
        args.set(key, decode_value(value));
    }
    args
}

fn encode_value(value: &ArgValue) -> String {
    match value {
        ArgValue::Bool(true) => "!true".to_string(),
        ArgValue::Bool(false) => "!false".to_string(),
        ArgValue::Null => "!null".to_string(),
        ArgValue::Num(n) => percent_encode(&format_num(*n)),
        ArgValue::Text(s) | ArgValue::Variant(s) => percent_encode(s),
        // `!,` then comma-joined items. Safe because `percent_encode` escapes a
        // comma inside an item to `%2C`, so the separator cannot collide with
        // content. Nested lists are flattened; the panel has no widget for them
        // and the URL is not the place to invent one.
        ArgValue::List(items) => {
            let body = items
                .iter()
                .map(encode_value)
                .collect::<Vec<_>>()
                .join(",");
            format!("{LIST_PREFIX}{body}")
        }
    }
}

fn decode_value(raw: &str) -> ArgValue {
    match raw {
        "!true" => return ArgValue::Bool(true),
        "!false" => return ArgValue::Bool(false),
        "!null" => return ArgValue::Null,
        _ => {}
    }
    if let Some(body) = raw.strip_prefix(LIST_PREFIX) {
        return ArgValue::List(
            body.split(',')
                .filter(|s| !s.is_empty())
                .map(decode_value)
                .collect(),
        );
    }
    let decoded = percent_decode(raw);
    match decoded.parse::<f64>() {
        Ok(n) if n.is_finite() => ArgValue::Num(n),
        _ => ArgValue::Text(decoded),
    }
}

/// The marker that opens a list value. See [`encode_args`].
const LIST_PREFIX: &str = "!,";

/// Percent-encode everything outside the RFC 3986 unreserved set.
pub fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Reverse [`percent_encode`]. Invalid escapes are passed through literally.
pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
