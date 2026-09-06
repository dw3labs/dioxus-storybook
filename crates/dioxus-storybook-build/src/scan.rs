//! Finding `#[story]` functions in a source file.
//!
//! This is a deliberate non-parse: pulling `syn` into every downstream build
//! script to answer "which functions are annotated" is not worth it. What it
//! *is* careful about is not being fooled by text that only looks like code —
//! `#[story]` inside a string literal, a doc comment, or a commented-out block
//! must not register a story.

/// Blank out comments and string/char literal contents, preserving byte length
/// so that later offsets still line up with the original source.
fn strip_noise(src: &str) -> String {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Code,
        LineComment,
        BlockComment(usize),
        Str,
        RawStr(usize),
        Char,
    }

    let bytes = src.as_bytes();
    let mut out = vec![b' '; bytes.len()];
    let mut state = State::Code;
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        match state {
            State::Code => {
                // Raw strings: r"..", r#".."#, br#".."#
                let raw_start = {
                    let mut j = i;
                    if bytes[j] == b'b' {
                        j += 1;
                    }
                    if bytes.get(j) == Some(&b'r') {
                        let mut hashes = 0;
                        let mut k = j + 1;
                        while bytes.get(k) == Some(&b'#') {
                            hashes += 1;
                            k += 1;
                        }
                        if bytes.get(k) == Some(&b'"') {
                            Some((k + 1, hashes))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some((after_quote, hashes)) = raw_start {
                    state = State::RawStr(hashes);
                    i = after_quote;
                    continue;
                }
                if b == b'/' && next == Some(b'/') {
                    state = State::LineComment;
                    i += 2;
                    continue;
                }
                if b == b'/' && next == Some(b'*') {
                    state = State::BlockComment(1);
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    state = State::Str;
                    i += 1;
                    continue;
                }
                if b == b'\'' {
                    // A lifetime (`'a`) is not a char literal. Char literals are
                    // at most a few bytes and always close with a quote.
                    let closes = bytes[i + 1..]
                        .iter()
                        .take(5)
                        .position(|c| *c == b'\'')
                        .is_some();
                    if closes {
                        state = State::Char;
                        i += 1;
                        continue;
                    }
                }
                out[i] = b;
                i += 1;
            }
            State::LineComment => {
                if b == b'\n' {
                    out[i] = b;
                    state = State::Code;
                }
                i += 1;
            }
            State::BlockComment(depth) => {
                if b == b'/' && next == Some(b'*') {
                    state = State::BlockComment(depth + 1);
                    i += 2;
                } else if b == b'*' && next == Some(b'/') {
                    state = if depth == 1 {
                        State::Code
                    } else {
                        State::BlockComment(depth - 1)
                    };
                    i += 2;
                } else {
                    if b == b'\n' {
                        out[i] = b;
                    }
                    i += 1;
                }
            }
            State::Str | State::Char => {
                let closer = if state == State::Str { b'"' } else { b'\'' };
                if b == b'\\' {
                    i += 2;
                    continue;
                }
                if b == closer {
                    state = State::Code;
                }
                if b == b'\n' {
                    out[i] = b;
                }
                i += 1;
            }
            State::RawStr(hashes) => {
                if b == b'"' {
                    let closed = (1..=hashes).all(|h| bytes.get(i + h) == Some(&b'#'));
                    if closed {
                        state = State::Code;
                        i += hashes + 1;
                        continue;
                    }
                }
                if b == b'\n' {
                    out[i] = b;
                }
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Names of every function carrying a `#[story]` attribute, in source order.
///
/// ```
/// # use dioxus_storybook_build::story_fns;
/// let src = r#"
///     #[story]
///     fn primary() -> ButtonProps { todo!() }
///
///     #[story(name = "With Icon")]
///     pub fn with_icon() -> ButtonProps { todo!() }
/// "#;
/// assert_eq!(story_fns(src), ["primary", "with_icon"]);
/// ```
pub fn story_fns(src: &str) -> Vec<String> {
    let cleaned = strip_noise(src);
    let bytes = cleaned.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        // Look for `#[story` with `story` as a whole token.
        if bytes[i] != b'#' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while bytes.get(j).is_some_and(|b| b.is_ascii_whitespace()) {
            j += 1;
        }
        if bytes.get(j) != Some(&b'[') {
            i += 1;
            continue;
        }
        j += 1;
        while bytes.get(j).is_some_and(|b| b.is_ascii_whitespace()) {
            j += 1;
        }
        if !cleaned[j..].starts_with("story") {
            i += 1;
            continue;
        }
        let after = j + "story".len();
        if bytes.get(after).copied().is_some_and(is_ident_byte) {
            // `#[story_meta]` or similar — not us.
            i += 1;
            continue;
        }

        // Skip the attribute's balanced brackets.
        let mut depth = 0i32;
        let mut k = j - 1; // at the '['
        while k < bytes.len() {
            match bytes[k] {
                b'[' | b'(' | b'{' => depth += 1,
                b']' | b')' | b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        k += 1;
                        break;
                    }
                }
                _ => {}
            }
            k += 1;
        }

        // Scan forward for the `fn` keyword, then take the identifier.
        if let Some(name) = next_fn_name(&cleaned, k) {
            found.push(name);
        }
        i = k;
    }

    found
}

fn next_fn_name(src: &str, from: usize) -> Option<String> {
    let bytes = src.as_bytes();
    let mut i = from;
    while i + 2 <= bytes.len() {
        if &src[i..i + 2] == "fn"
            && (i == 0 || !is_ident_byte(bytes[i - 1]))
            && !bytes.get(i + 2).copied().is_some_and(is_ident_byte)
        {
            let mut j = i + 2;
            while bytes.get(j).is_some_and(|b| b.is_ascii_whitespace()) {
                j += 1;
            }
            let start = j;
            while bytes.get(j).copied().is_some_and(is_ident_byte) {
                j += 1;
            }
            if j > start {
                return Some(src[start..j].to_string());
            }
            return None;
        }
        // Give up at the next item boundary so a stray attribute cannot capture
        // a function twenty lines away.
        if bytes[i] == b'{' || bytes[i] == b';' {
            return None;
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_plain_and_parameterised_attributes() {
        let src = "#[story]\nfn a() -> P {}\n#[story(name = \"B!\")]\npub fn b() -> P {}\n";
        assert_eq!(story_fns(src), ["a", "b"]);
    }

    #[test]
    fn ignores_lookalikes_in_comments_and_strings() {
        let src = r###"
            // #[story] fn commented() -> P {}
            /* #[story] fn blocked() -> P {} */
            /// #[story] fn documented() -> P {}
            const S: &str = "#[story] fn stringy() -> P {}";
            const R: &str = r#"#[story] fn rawstringy() -> P {}"#;
            #[story]
            fn real() -> P {}
        "###;
        assert_eq!(story_fns(src), ["real"]);
    }

    #[test]
    fn ignores_other_attributes_with_a_story_prefix() {
        let src = "#[story_meta]\nfn nope() {}\n#[storybook]\nfn also_nope() {}\n";
        assert!(story_fns(src).is_empty());
    }

    #[test]
    fn tolerates_intervening_attributes_and_doc_comments() {
        let src = "#[story]\n#[allow(dead_code)]\n/// docs\nasync fn c() -> P {}\n";
        assert_eq!(story_fns(src), ["c"]);
    }

    #[test]
    fn does_not_reach_across_an_item_boundary() {
        let src = "#[story]\nstruct NotAFn;\nfn later() -> P {}\n";
        assert!(story_fns(src).is_empty());
    }

    #[test]
    fn survives_lifetimes_and_char_literals() {
        let src = "fn f<'a>(x: &'a str) -> char { '\\'' }\n#[story]\nfn real() -> P {}\n";
        assert_eq!(story_fns(src), ["real"]);
    }
}
