# 0006 — M3 part 2: globals, the toolbar, and the first parameter

**Date:** 2026-09-06 · **Status:** M3 5 of 6 items complete
**Toolchain:** rustc 1.97.1 · dx 0.7.10 · dioxus 0.7.10 · macOS aarch64

Globals — the values the toolbar selects and every story sees. Plus the first
thing that actually *reads* a parameter, because the session before this one
built the merge and left nothing consuming it.

**Delivered:** 150 tests + 13 doctests, all green. `rust-analyzer diagnostics .`
clean. Verified in a browser, clicking it.

---

## What was built

| | Delivered as |
|---|---|
| `GlobalType` — a toolbar declaration | `core/src/globals.rs` |
| `globals::resolve` / `is_default` / `GlobalType::coerce` | same |
| `Project::with_globals` / `resolved_globals` | `core/src/story.rs` |
| `StoryContext::globals` | same |
| `Event::SetGlobals` + its wire form | `core/src/channel.rs`, `core/src/wire.rs` |
| `&globals=theme:dark` in the URL | `core/src/url.rs` |
| The toolbar itself | `ui/src/toolbar.rs` |
| `parameters.layout` — centred / padded / fullscreen | `ui/src/lib.rs`, `ui/src/style.rs` |
| Tests | `dioxus-storybook/tests/decorators.rs` (+8), `ui/tests/two_documents.rs` (+8), `core/tests/url.rs` (+3) |
| A theme and a grid overlay in the example | `examples/button-gallery/src/main.rs` |

---

## Findings

### G1 — Globals are not a fourth vocabulary. They are args, declared like props.

There were already two value languages in this project, and adding a third for
globals was the obvious move and the wrong one:

| | changes at run time | scope | declared by |
|---|---|---|---|
| `ArgValue` args | yes | one story | the props type |
| `ParamValue` parameters | no | a level | project / meta / story |
| globals | yes | the whole book | the project |

Globals sit in the same cell as args on every axis that matters — they change at
run time and they need a widget. So a `GlobalType` declares a [`Control`], the
same enum a prop's `ArgType` does, and the current values are an `ArgMap`.

That is not an economy of types, it is an economy of *code*: globals got a
control widget, a URL encoding and a wire encoding without a line of new work in
any of the three. `Event::SetGlobals` carries an `ArgMap` that `wire.rs` already
knew how to encode; `?globals=…` uses the same `encode_args` as `?args=…`.

### G2 — And that inheritance brings the URL codec's untypedness with it

The URL codec is deliberately untyped (a documented M1 property): `theme:dark`
decodes to `ArgValue::Text`, because a query string cannot know it was a
`Variant`. Args survive that because the generated applier restores the type on
the way out.

Globals had no such step, and the bug it produces is nasty precisely because it
is invisible in the common case: a decorator matching

```rust
ctx.globals().get("theme") == Some(&ArgValue::Variant("dark".into()))
```

works when the toolbar was clicked and silently fails when the same state
arrives from a pasted link. Caught by a test that round-tripped a `UrlState` and
got a different `ArgValue` shape back.

So `GlobalType::coerce` is the applier's analogue: `Select`/`Radio` → `Variant`,
`Toggle` → `Bool`, `Number`/`Range` → `Num`, and anything unconvertible falls
back to the declared default rather than failing, because the input is a link
somebody pasted. `resolve` runs it over every declared name.

The general shape, worth carrying forward: **anything that enters through the
URL is untyped text and needs a declared-shape pass before it is compared.**

### G3 — Only selections travel. Defaults are filled in on both sides.

`Event::SetGlobals` carries only the globals that have been moved off their
default, and the URL carries only those too. Each half fills in the declarations
itself, which it can do because they are `&'static` and both halves share the
bundle — the same property that made the M3 split cheap.

That is not just smaller messages. It makes an *empty selection map* mean
exactly "nothing has been changed", which is the question a reset button is
asking. Setting a global back to its declared value therefore **removes** the
key rather than storing it (`globals::is_default`), so the reset button
disables itself again and the link stops carrying values nobody chose.

Verified in the browser: selecting sepia gives `?…&globals=theme:sepia`, and
reset removes the parameter from the URL entirely.

### G4 — A `contains` assertion matched the stylesheet, twice, in one file

`assert!(manager_html.contains("dxsb-globals"))` passes whether the toolbar
rendered or not — the manager inlines its whole stylesheet, and the stylesheet
names every class it styles. Both the positive and the negative test were wrong;
the negative one is what failed and gave it away.

Fixed by matching `class="dxsb-globals"`. This is the second sighting of the
same trap (LOG-0005 / F7 had `"dioxus-storybook"` containing `"ok"`), so it is
now a standing fact: **assert on markup that cannot appear by accident.**

### G5 — A test's subject became its opposite one session later

`an_unknown_tag_from_a_newer_peer_is_dropped_not_fatal` asserted that
`dxsb1|manager|globals|3:abc` decodes to `None`. One session after it was
written, `globals` became a real tag — and the test would then have been
asserting that a *valid* message is dropped, while still passing for a while
because the field count did not match.

The tag is now `no-such-tag-will-ever-exist`. When a test's premise is "this
does not exist yet", say so in a way that cannot quietly stop being true.

### G6 — Canvas padding is surface a decorator cannot paint

The theme decorator in the example painted a dark background and it appeared as
a dark rectangle inset by 40px of white — the canvas's own padding.

That is what `parameters.layout` is for, and the example had already been
carrying `layout: "centered"` since the previous session with nothing reading
it. The preview now reads it: `centered` (default), `padded`, `fullscreen`. It
is the first consumer of `ResolvedParameters` and the proof that the merge built
last session is wired to something.

An unrecognised value falls back to `centered` rather than failing. Parameters
are an open, stringly-typed space shared with addons this build may not have; a
typo, or a value meant for a newer addon, must not be the difference between
seeing a story and seeing nothing.

### G7 — Changing a global must not remount the story

`StoryHost` is keyed on the story id, so a globals change re-renders it in place
rather than remounting. That is the wanted behaviour and it is worth being
explicit about: flipping the theme should not reset a story's hook state, and it
does not, because the key did not change.

---

## Decisions

| # | Decision | Why |
|---|---|---|
| — | Globals reuse `Control` + `ArgMap` rather than a new vocabulary | G1 |
| — | `coerce` on the way out, mirroring the args applier | G2 |
| — | Only selections on the wire and in the URL; defaults filled in per side | G3 |
| — | A selection equal to the default is removed, not stored | G3 — it is what makes "changed" answerable |
| — | Globals are declared on the `Project` only, not per story | A global that only some stories have is a prop with extra steps |
| — | A `Radio` global renders as a dropdown in the toolbar | The declaration is "one of a fixed set"; which of the two shapes renders it is the surface's decision, and a radio group does not fit a toolbar row |
| — | `SetGlobals` is emitted even with no story selected | A book with nothing selected still has a theme |

---

## Files produced

```
crates/dioxus-storybook-core/src/globals.rs   GlobalType, resolve, coerce, is_default
crates/dioxus-storybook-ui/src/toolbar.rs     the toolbar
```

Changed: `core/src/channel.rs` (`SetGlobals`), `core/src/wire.rs`,
`core/src/url.rs` (`globals=`), `core/src/story.rs` (`Project::with_globals`,
`StoryContext::globals`, `render_decorated` takes globals), `core/src/lib.rs`,
`ui/src/lib.rs` (globals state both sides, the toolbar, `parameters.layout`),
`ui/src/style.rs`, `dioxus-storybook/src/lib.rs` (docs + re-exports), the three
test files, and `examples/button-gallery/src/main.rs`.

---

## Notes for the next session

**One M3 item is left:** the environment addons — viewport, backgrounds,
measure, outline. All four are parameter-driven, and `parameters.layout` is the
worked example of how one reads its parameter and what to do with a value it
does not recognise.

`viewport` is the interesting one and it is nearly free: the preview is a frame
whose width the shell owns, so a viewport addon is a class on `.dxsb-frame` plus
a picker that looks exactly like the globals toolbar. It may well *be* a global
rather than a parameter — Storybook treats the selected viewport as globals
state and the available list as a parameter, which now maps cleanly onto both
mechanisms this project has.

`backgrounds` overlaps with what the example's theme decorator does by hand.
Worth deciding whether it is a built-in addon or just the documented decorator
pattern before writing it.

The `ErrorBoundary` for `Element`-level errors is also still open, and is
distinct from the panic hook that shipped in 0005.
