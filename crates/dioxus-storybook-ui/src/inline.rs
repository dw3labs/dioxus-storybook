//! Rendering the inline part of a doc comment.
//!
//! A `///` is rustdoc, and rustdoc is Markdown. The storybook quotes those
//! comments in three places — a component's description, a story's blurb, the
//! props table's description column — and quoting them *raw* puts `` `oninput` ``
//! and `**controlled**` on the page verbatim, which reads as a typo rather than
//! as emphasis.
//!
//! The answer here is deliberately not a Markdown renderer. Three constructs
//! account for essentially all of the inline markup a Rust doc comment carries,
//! and each maps onto one element:
//!
//! | | becomes |
//! |---|---|
//! | `` `code` `` | `<code>` |
//! | `**strong**` | `<strong>` |
//! | `*emphasis*`, `_emphasis_` | `<em>` |
//! | `[label](target)`, ``[`label`]`` | just the label |
//!
//! Intra-doc links lose their target on purpose: it points at a Rust item, and
//! there is nowhere in a storybook to send someone for one.
//!
//! Anything block-shaped — headings, lists, fenced code, tables — is out of
//! scope and passes through as text. It rarely arrives, because the macro only
//! captures a doc comment's **summary paragraph**.

use dioxus::prelude::*;

/// One run of doc-comment text, with the one bit of formatting it carries.
#[derive(Debug, Clone, PartialEq)]
enum Piece {
    Text(String),
    Code(String),
    Strong(String),
    Em(String),
}

/// Render a doc comment's inline markup.
///
/// Produces plain text and nothing else when there is no markup, which is the
/// common case.
#[component]
pub fn Doc(text: String) -> Element {
    let pieces = parse(&text);
    rsx! {
        for (i, piece) in pieces.into_iter().enumerate() {
            match piece {
                Piece::Text(s) => rsx! { Fragment { key: "{i}", "{s}" } },
                Piece::Code(s) => rsx! { code { key: "{i}", class: "dxsb-doccode", "{s}" } },
                Piece::Strong(s) => rsx! { strong { key: "{i}", "{s}" } },
                Piece::Em(s) => rsx! { em { key: "{i}", "{s}" } },
            }
        }
    }
}

/// Flatten `[label](target)` and `` [`label`] `` down to their label.
///
/// Done as a pass over the string rather than in the scanner because a label may
/// itself be code — ``[`use_args`]`` is the shape almost every intra-doc link in
/// this codebase takes — and flattening first lets the scanner see the backticks.
fn flatten_links(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            // Not a link: copy this whole char, which may be multi-byte.
            let ch = text[i..].chars().next().expect("i is a char boundary");
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }
        // `[` with a matching `]` and no nesting is a link; anything else is
        // literal text — `&[T]` in a doc comment must survive unchanged.
        let Some(close) = text[i + 1..].find(']').map(|n| i + 1 + n) else {
            out.push('[');
            i += 1;
            continue;
        };
        let label = &text[i + 1..close];
        if label.is_empty() || label.contains('[') {
            out.push('[');
            i += 1;
            continue;
        }
        out.push_str(label);
        i = close + 1;
        // An optional `(target)` immediately after, which is dropped entirely.
        if text[i..].starts_with('(')
            && let Some(n) = text[i..].find(')')
        {
            i += n + 1;
        }
    }
    out
}

/// Split doc text into runs, innermost-first: code, then strong, then emphasis.
fn parse(text: &str) -> Vec<Piece> {
    let flat = flatten_links(text);
    let mut out = Vec::new();
    for run in split_on(&flat, "`", "`", Piece::Code) {
        match run {
            // Only *plain* runs are looked at again: `**` inside a code span is
            // two asterisks, not emphasis.
            Piece::Text(plain) => {
                for run in split_on(&plain, "**", "**", Piece::Strong) {
                    match run {
                        Piece::Text(plain) => {
                            out.extend(split_on(&plain, "*", "*", Piece::Em));
                        }
                        other => out.push(other),
                    }
                }
            }
            other => out.push(other),
        }
    }
    // `_x_` is the other emphasis spelling, and the one that needs a guard:
    // `some_ident_name` must not become emphasis, so it only counts when the
    // delimiters sit at a word boundary.
    out.into_iter()
        .flat_map(|piece| match piece {
            Piece::Text(plain) => split_underscores(&plain),
            other => vec![other],
        })
        .filter(|p| !matches!(p, Piece::Text(s) if s.is_empty()))
        .collect()
}

/// Split on a delimiter pair, wrapping what is between them with `wrap`.
///
/// An unclosed delimiter is literal text, so a lone backtick or a multiplication
/// sign does not swallow the rest of the sentence.
fn split_on(text: &str, open: &str, close: &str, wrap: fn(String) -> Piece) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        let after = &rest[start + open.len()..];
        let Some(end) = after.find(close) else { break };
        if end == 0 {
            // An empty span (`` `` ``, `****`): not markup, keep it as text.
            let keep = start + open.len() + close.len();
            out.push(Piece::Text(rest[..keep].to_string()));
            rest = &rest[keep..];
            continue;
        }
        out.push(Piece::Text(rest[..start].to_string()));
        out.push(wrap(after[..end].to_string()));
        rest = &after[end + close.len()..];
    }
    out.push(Piece::Text(rest.to_string()));
    out
}

/// `_emphasis_`, but only when the underscores are not inside a word.
fn split_underscores(text: &str) -> Vec<Piece> {
    let chars: Vec<char> = text.chars().collect();
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    while i < chars.len() {
        let opens = chars[i] == '_'
            && (i == 0 || !is_word(chars[i - 1]))
            && chars.get(i + 1).is_some_and(|c| !c.is_whitespace() && *c != '_');
        if !opens {
            buf.push(chars[i]);
            i += 1;
            continue;
        }
        let close = (i + 1..chars.len()).find(|&j| {
            chars[j] == '_' && !chars.get(j + 1).copied().is_some_and(is_word)
        });
        match close {
            Some(j) => {
                out.push(Piece::Text(std::mem::take(&mut buf)));
                out.push(Piece::Em(chars[i + 1..j].iter().collect()));
                i = j + 1;
            }
            None => {
                buf.push(chars[i]);
                i += 1;
            }
        }
    }
    out.push(Piece::Text(buf));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Vec<Piece> {
        parse(s)
    }

    #[test]
    fn plain_prose_is_one_piece() {
        assert_eq!(text("Just words."), vec![Piece::Text("Just words.".into())]);
    }

    #[test]
    fn code_spans_become_code() {
        assert_eq!(
            text("the `value` arg"),
            vec![
                Piece::Text("the ".into()),
                Piece::Code("value".into()),
                Piece::Text(" arg".into()),
            ]
        );
    }

    #[test]
    fn strong_wins_over_emphasis() {
        assert_eq!(text("a **bold** one"), vec![
            Piece::Text("a ".into()),
            Piece::Strong("bold".into()),
            Piece::Text(" one".into()),
        ]);
        assert_eq!(text("an *italic* one"), vec![
            Piece::Text("an ".into()),
            Piece::Em("italic".into()),
            Piece::Text(" one".into()),
        ]);
    }

    #[test]
    fn links_keep_their_label_and_lose_their_target() {
        assert_eq!(
            text("see [`use_args`](crate::use_args) for that"),
            vec![
                Piece::Text("see ".into()),
                Piece::Code("use_args".into()),
                Piece::Text(" for that".into()),
            ]
        );
        assert_eq!(
            text("see [Controlled] instead"),
            vec![Piece::Text("see Controlled instead".into())]
        );
    }

    /// The failure that matters: markup inside a code span is not markup, and a
    /// slice type is not a link.
    #[test]
    fn code_and_brackets_are_left_alone() {
        assert_eq!(text("`a * b`"), vec![Piece::Code("a * b".into())]);
        assert_eq!(text("`**not bold**`"), vec![Piece::Code("**not bold**".into())]);
    }

    #[test]
    fn an_unclosed_delimiter_is_literal() {
        assert_eq!(text("2 * 3 = 6"), vec![Piece::Text("2 * 3 = 6".into())]);
        assert_eq!(text("a lone ` tick"), vec![Piece::Text("a lone ` tick".into())]);
    }

    /// `some_ident_name` is not three words in italics.
    #[test]
    fn underscores_inside_a_word_are_not_emphasis() {
        assert_eq!(
            text("call some_ident_name here"),
            vec![Piece::Text("call some_ident_name here".into())]
        );
        assert_eq!(text("an _italic_ word"), vec![
            Piece::Text("an ".into()),
            Piece::Em("italic".into()),
            Piece::Text(" word".into()),
        ]);
    }
}
