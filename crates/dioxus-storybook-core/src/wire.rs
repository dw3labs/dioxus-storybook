//! The over-the-wire form of an [`Event`].
//!
//! From M3 the manager and the preview are two *documents*: the preview runs in
//! an iframe and the two halves talk over `postMessage`, which carries strings,
//! not Rust values. This module is that codec, and it is deliberately in `core`
//! and free of `web-sys` so it can be tested on the host.
//!
//! # Why a hand-rolled format
//!
//! [`Event`] is a closed vocabulary of nine messages whose payloads are strings
//! and [`ArgMap`]s, and `ArgMap` already has a tested textual encoding — the one
//! the URL uses. Pulling in `serde` + a JSON codec to move nine tags would add
//! two dependencies to a crate that currently has one, for no capability.
//!
//! # Format
//!
//! ```text
//! dxsb1|<sender>|<tag>[|<byte-len>:<bytes>]*
//! ```
//!
//! Fields are length-prefixed, so a payload may contain `|`, `:`, newlines or
//! any other byte without escaping — which matters, because an action payload
//! is arbitrary `Debug` output.
//!
//! # Two things the envelope is load-bearing for
//!
//! - **`window.onmessage` is a shared bus.** Browser extensions, embedded
//!   widgets and dev tooling all post messages at the page. Anything that does
//!   not start with [`MAGIC`] is not ours and is dropped.
//! - **A document can hear itself.** `window.parent` is `window` when a page is
//!   *not* framed, so a preview opened directly would otherwise receive its own
//!   messages and echo forever. The sender's [`ViewMode`] is on the wire and
//!   [`decode`] takes the mode it is willing to accept.
//!
//! # Known lossiness
//!
//! `ArgMap` travels through [`crate::url::encode_args`], which cannot tell an
//! empty list from a list holding one empty string. That is a documented
//! property of the URL codec and is inherited here rather than introduced.

use crate::url::{decode_args, encode_args};
use crate::Event;

/// The envelope prefix. Present on every message this crate sends, so a
/// listener can ignore the rest of the page's `postMessage` traffic.
pub const MAGIC: &str = "dxsb1";

/// The query-string parameter that selects a document's role, spelled as
/// Storybook spells its own.
pub const VIEW_MODE_PARAM: &str = "viewMode";

/// Which half of the storybook a document is running.
///
/// One bundle serves both: the manager decides which role it is in from its own
/// URL, exactly as an iframe would. See [`crate::channel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ViewMode {
    /// The shell: sidebar, toolbar, addon panels. Owns the iframe.
    Manager,
    /// The story canvas, alone in its own document.
    Preview,
}

impl ViewMode {
    /// The name used in the URL and on the wire.
    pub const fn as_str(self) -> &'static str {
        match self {
            ViewMode::Manager => "manager",
            ViewMode::Preview => "preview",
        }
    }

    /// The other half. Every message a document sends is addressed to it.
    pub const fn peer(self) -> Self {
        match self {
            ViewMode::Manager => ViewMode::Preview,
            ViewMode::Preview => ViewMode::Manager,
        }
    }

    /// Parse a mode name. Anything unrecognised — including the absent
    /// parameter — is [`ViewMode::Manager`], because a bare URL is the shell.
    pub fn parse(raw: &str) -> Self {
        match raw {
            "preview" => ViewMode::Preview,
            _ => ViewMode::Manager,
        }
    }

    /// The mode a query string asks for, via [`VIEW_MODE_PARAM`].
    ///
    /// This is how a document learns which half of the storybook it is: the
    /// manager points its iframe at its own URL with the parameter added.
    ///
    /// ```
    /// # use dioxus_storybook_core::ViewMode;
    /// assert_eq!(ViewMode::from_query("?id=a--b&viewMode=preview"), ViewMode::Preview);
    /// assert_eq!(ViewMode::from_query("?id=a--b"), ViewMode::Manager);
    /// ```
    pub fn from_query(query: &str) -> Self {
        query
            .strip_prefix('?')
            .unwrap_or(query)
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == VIEW_MODE_PARAM)
            .map_or(ViewMode::Manager, |(_, value)| ViewMode::parse(value))
    }
}

/// Render `event` as a string that [`decode`] can read back.
///
/// `from` is the sender's own mode, not the recipient's.
///
/// ```
/// # use dioxus_storybook_core::{Event, wire::{encode, decode, ViewMode}};
/// let sent = Event::StoryRendered { id: "forms-button--primary".into() };
/// let raw = encode(ViewMode::Preview, &sent);
/// // read by the manager, whose peer is the preview
/// assert_eq!(decode(ViewMode::Manager, &raw), Some(sent));
/// ```
pub fn encode(from: ViewMode, event: &Event) -> String {
    // No wildcard arm on purpose. `Event` is `#[non_exhaustive]` for
    // downstream crates but exhaustive in here, so a new variant fails to
    // compile until someone gives it a wire form — which is the only way to
    // stop a message from silently going nowhere once the transport is real.
    let (tag, fields): (&str, Vec<String>) = match event {
        Event::SetCurrentStory { id } => ("story", vec![id.clone()]),
        Event::UpdateArgs { id, args } => ("args", vec![id.clone(), encode_args(args)]),
        Event::RequestArgsUpdate { id, args } => {
            ("argsreq", vec![id.clone(), encode_args(args)])
        }
        Event::StoryPrepared { id, initial_args } => {
            ("prepared", vec![id.clone(), encode_args(initial_args)])
        }
        Event::ResetArgs { id } => ("reset", vec![id.clone()]),
        Event::StoryRendered { id } => ("rendered", vec![id.clone()]),
        Event::StoryMissing { id } => ("missing", vec![id.clone()]),
        Event::PreviewPanicked { message } => ("panic", vec![message.clone()]),
        Event::ActionLogged { name, payload } => ("action", vec![name.clone(), payload.clone()]),
        Event::PreviewReady => ("ready", Vec::new()),
    };

    let mut out = format!("{MAGIC}|{}|{tag}", from.as_str());
    for field in fields {
        out.push('|');
        out.push_str(&field.len().to_string());
        out.push(':');
        out.push_str(&field);
    }
    out
}

/// Read a message back, or `None` if it is not one for us.
///
/// `receiver` is the reader's *own* mode. A message is accepted only if its
/// sender is that mode's [`peer`](ViewMode::peer) — which is what keeps an
/// unframed preview, whose `window.parent` is itself, from answering its own
/// messages forever.
pub fn decode(receiver: ViewMode, raw: &str) -> Option<Event> {
    let rest = raw.strip_prefix(MAGIC)?.strip_prefix('|')?;
    let (sender, rest) = rest.split_once('|')?;
    if sender != receiver.peer().as_str() {
        return None;
    }
    // The `|` before the first field is kept: `read_fields` expects to find one
    // in front of every field, including the first.
    let (tag, rest) = match rest.find('|') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let fields = read_fields(rest)?;

    let one = || fields.first().cloned();
    let two = || Some((fields.first()?.clone(), fields.get(1)?.clone()));

    Some(match tag {
        "story" => Event::SetCurrentStory { id: one()? },
        "args" => {
            let (id, args) = two()?;
            Event::UpdateArgs { id, args: decode_args(&args) }
        }
        "argsreq" => {
            let (id, args) = two()?;
            Event::RequestArgsUpdate { id, args: decode_args(&args) }
        }
        "prepared" => {
            let (id, args) = two()?;
            Event::StoryPrepared { id, initial_args: decode_args(&args) }
        }
        "reset" => Event::ResetArgs { id: one()? },
        "rendered" => Event::StoryRendered { id: one()? },
        "missing" => Event::StoryMissing { id: one()? },
        "action" => {
            let (name, payload) = two()?;
            Event::ActionLogged { name, payload }
        }
        "panic" => Event::PreviewPanicked { message: one()? },
        "ready" => Event::PreviewReady,
        // A newer peer sending a message this build has never heard of. Drop it
        // rather than fail: the two documents are the same bundle today, but a
        // stale cached iframe is a real thing.
        _ => return None,
    })
}

/// Split `|len:bytes` repetitions. `None` on anything malformed.
fn read_fields(mut rest: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    while !rest.is_empty() {
        rest = rest.strip_prefix('|')?;
        let (len, tail) = rest.split_once(':')?;
        let len: usize = len.parse().ok()?;
        // `get` on a str returns None rather than panicking when the range
        // splits a UTF-8 sequence, so a corrupt length is caught here.
        let field = tail.get(..len)?;
        fields.push(field.to_string());
        rest = &tail[len..];
    }
    Some(fields)
}
