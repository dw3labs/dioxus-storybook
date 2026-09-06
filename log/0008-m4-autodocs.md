# 0008 — M4: autodocs

**Date:** 2026-09-06
**Goal:** M4 — a generated documentation page per component.
**Outcome:** shipped. 195 tests + doctests green, `rust-analyzer diagnostics .`
clean, `cargo doc --workspace` clean (it was not, before — see F6), verified by
hand in the browser.

---

## Goal

`docs/PLAN.md` defines M4 as five things: an autodocs page per component, a
props table from `ArgType` + doc comments, a story source snippet, a description
from doc comments, and a docs/canvas toggle. All five shipped.

## What was done

- **`dioxus-storybook-core/src/docs.rs`** — the whole model: `Autodocs`
  (`Always` / `Tagged` / `Never`), `DocsPage`, `Entry`, `id_for`, `resolve`,
  `entry`, `id_for_story`, `canvas_id_for`.
- **Capture, all in the macros.** `#[story]` now reads the function's `///` and
  the *source text* of its body; `story_meta!` gained `description:`;
  `#[derive(Controls)]` emits `Controllable::DOCS` from the props type's `///`.
- **`StoryDef`** gained `docs()`, `source()`, `component_docs()`;
  **`Meta`** gained `description()`; **`Project`** gained `autodocs()`.
- **`StoryView`** — `Canvas` or `Docs`, on `StoryContext`. See F3; this is the
  one thing autodocs asks of a decorator author.
- **`dioxus-storybook-ui/src/docs.rs`** — `DocsView`, rendered in the *preview*.
- **`StoryHost`** gained `reporting` and `view`, and a local args overlay (F4).
- **The toolbar became a three-column grid** so the Canvas/Docs toggle stops
  moving (F5).
- **`examples/button-gallery`** — descriptions, story doc comments, and a
  `ctx.view()` branch in the project decorator that demonstrates F3.
- 17 new tests in `crates/dioxus-storybook/tests/autodocs.rs`, 4 in
  `two_documents.rs`.

## Findings

### F1 — A docs page is an **id**, not a mode. Nothing new on the wire.

The obvious shape was a second view mode: a `viewMode=docs` parameter, a
`SetViewMode` message, a flag in the preview. None of that was built, and none
of it was needed.

`forms-button--docs` is an entry in the **same id space** as
`forms-button--primary`. It therefore travels in the existing `?id=` parameter,
in the existing `Event::SetCurrentStory`, and renders in the existing iframe.
The only new code on the path is `docs::entry`, which asks the registry for a
story first and falls back to building a `DocsPage`.

Asking for the story first is not an optimisation — it is the **collision rule**.
A story literally named "Docs" produces the id a docs page wants, and the
author's own story is the one that should win. `a_real_story_wins_a_collision_
with_a_docs_id` pins it.

The wire diff for this milestone is empty. That is the finding.

### F2 — The docs page renders in the preview, and it could not be otherwise.

Every example on a docs page is a real story rendered by the same
`render_decorated` the canvas uses. That forces the page into the preview
document, for three independent reasons:

- evaluating props builds `EventHandler`s and needs a live Dioxus scope;
- decorators are what put the author's stylesheet *inside* the frame;
- the standing rule that the manager renders no story and calls no `base_args`.

A docs page drawn by the manager would be a page of components styled by the
workbench, which is the exact failure the M3 split exists to prevent. The
consequence is that the page's own chrome — headings, props table, source
blocks — lives in `PREVIEW_CSS`, not in the shell's stylesheet.

### F3 — `100vh` in a decorator is right on the canvas and wrong on a docs page.

Found in the browser, immediately, and it is the one thing autodocs asks of a
decorator author. The example's project decorator paints the theme with
`min-height:100vh`, which is exactly correct when the story owns the document.
On a docs page a dozen examples are stacked in a column, and every one of them
became a screen tall.

There is no CSS fix: `vh` is the viewport, and no containing block redefines it.
So the decorator has to be told, and `StoryContext::view()` is how. It is the
third time `StoryContext` has grown (globals, viewport, now this) and the third
time `#[non_exhaustive]` paid for itself.

Storybook has the same footgun and no better answer.

### F4 — A docs page has nowhere to put a story's args write-back.

`use_args` sends `RequestArgsUpdate`, the manager merges it into the current
arg set and re-broadcasts. On a docs page the "current entry" is the page, so a
write from one of a dozen examples would have landed in the manager's args and
gone into the address bar under a docs id.

Gating it off would make a controlled story on a docs page *inert* — typing does
nothing — which looks like a bug. So `StoryHost` keeps a **local overlay** when
`reporting` is false: the example is genuinely typeable, and the manager never
hears about it. On the canvas the overlay is never written and the manager stays
the single place that decides what the arg set is.

### F5 — A toolbar control that moves when you use it is a bug you cannot test.

The Canvas/Docs toggle was placed after the crumb. The viewport picker is hidden
on a docs page — so clicking "Docs" made the picker vanish and the toggle slide
sideways, out from under the cursor. Reported from the browser, by the user,
within minutes.

The fix is a **three-column grid**: `1fr auto 1fr`, with the crumb left, the
story id centred, and the tools right — the toggle last in that group, hard
against the edge. Only what is to the *left* of the toggle changes. The middle
column is exactly centred whatever sits on either side, which a flex row with a
spacer cannot promise.

Same lesson as LOG-0007/F4: a headless `VirtualDom` cannot deliver a click, and
nothing about this is expressible as an assertion on markup.

### F6 — `cargo doc` was already broken on main, and nothing noticed.

`cargo doc --no-deps -p dioxus-storybook-core` failed on an unresolved intra-doc
link in `wire.rs` — pre-existing, unrelated to M4, and invisible because nothing
in the loop runs `cargo doc`. For a crate whose whole milestone is documentation
and whose D1 ambition is publishing, that is worth catching: docs.rs would have
failed the build. Fixed, plus three redundant-link warnings.

`cargo doc --no-deps --workspace` is now clean and belongs in the definition of
done alongside `cargo test` and `rust-analyzer diagnostics .`.

### F7 — `Span::source_text()` gives the code as written. `quote!` does not.

The plan said the macro should capture "the rsx token stream as `&'static str`".
A token stream renders as `PillProps { outline : false , .. base () }` — a
rendering of the tokens, not the code anyone wrote.

`func.block.brace_token.span.join().source_text()` returns the **exact source
text** of the body: line breaks, indentation and comments included. Verified
with a throwaway proc macro before any of this was written. It is `Option`, and
`None` means the story itself came out of another macro, so there is a
token-stream fallback that is ugly but true.

`DelimSpan::join()` works on stable through proc-macro2. `Span::call_site()`
returns only the attribute text (`#[mac::capture]`), and the block's *open*
delimiter alone returns `{` — the join is what makes it work.

### F8 — The description has two sources, and the fallback is the useful one.

A proc macro cannot read the *component function's* doc comment; it only sees
the tokens it was invoked on. So `story_meta!` gained an explicit
`description:`. But the props type usually is where a component is explained,
and `#[derive(Controls)]` already reads that type — so `Controllable::DOCS`
carries its `///` and the docs page falls back to it.

That fallback is what makes a component documented with **no storybook-specific
prose at all**: the example's `Badge` has no `description:` and its page reads
"A small status pill." off `BadgeProps`. `DOCS` is an associated const with a
default body, so adding it to `Controllable` broke no hand-written impl — unlike
`wire_actions` in 0004.

### F9 — Autodocs defaults to *on*, which is not Storybook's default.

Storybook requires `tags: ["autodocs"]` per component. A feature nobody can find
is a feature nobody has, so `Autodocs::Always` is the default here and
`Autodocs::Tagged` reproduces Storybook's rule. `Never` removes the toggle and
the pages together.

The landing entry is still the first **story**, not the first docs page: a
component workbench that opens on prose is the wrong first impression.

## Decisions

| | Decision | Why |
|---|---|---|
| What a docs page is | **an entry id**, `kebab(title)--docs` | one id space; no URL parameter, no wire message, no second transport (F1) |
| Collisions | **a real story wins** | `--docs` is a legal story id and the author's story must stay reachable |
| Where it renders | **the preview document** | its examples are real stories with real decorators (F2) |
| Telling a decorator | **`StoryContext::view()`** | `100vh` has two right answers and no CSS can pick between them (F3) |
| A docs example's args | **a local overlay, never the manager** | the manager's args belong to the selected entry, which is the page (F4) |
| Reporting | **`StoryHost { reporting: false }`** | a dozen examples must not fight over the status bar and the controls panel |
| Toolbar layout | **a 3-column grid, toggle last** | the control you use to switch must not move when you switch (F5) |
| Source capture | **`Span::source_text`**, not `quote!` | the snippet is the code, not a rendering of its tokens (F7) |
| Description | **`story_meta!`, falling back to the props type's `///`** | a macro cannot see the component fn's doc comment (F8) |
| Autodocs default | **`Always`** | Storybook's opt-in tag hides the feature (F9) |
| Viewport on docs | **the picker is hidden, not ignored** | a control reading "Tablet" beside a full-width page is claiming something untrue |

## Files

```
crates/dioxus-storybook-core/src/docs.rs        NEW  Autodocs, DocsPage, Entry, resolve
crates/dioxus-storybook-core/src/story.rs       Meta::description, StoryDef::{docs,
                                                source, component_docs},
                                                Project::autodocs, StoryView
crates/dioxus-storybook-core/src/args.rs        Controllable::DOCS (defaulted)
crates/dioxus-storybook-core/src/lib.rs         module + re-exports
crates/dioxus-storybook-core/src/wire.rs        F6: broken intra-doc link
crates/dioxus-storybook-core/src/globals.rs     F6: redundant link targets
crates/dioxus-storybook-macro/src/story.rs      body_snippet + dedent, `description:`,
                                                story `///`, component docs
crates/dioxus-storybook-macro/src/props.rs      DOCS from the struct's `///`
crates/dioxus-storybook-ui/src/docs.rs          NEW  DocsView, DocsStory, PropsTable
crates/dioxus-storybook-ui/src/toolbar.rs       DocsToggle
crates/dioxus-storybook-ui/src/lib.rs           entry resolution, 3-column toolbar,
                                                StoryHost { reporting, view } + overlay
crates/dioxus-storybook-ui/src/panels.rs        the docs-mode controls message
crates/dioxus-storybook-ui/src/style.rs         .dxsb-docs*, .dxsb-docstoggle, grid toolbar
crates/dioxus-storybook-build/src/lib.rs        F6: broken intra-doc link
crates/dioxus-storybook/src/lib.rs              the "Autodocs" section; F6
crates/dioxus-storybook/tests/autodocs.rs       NEW  17 tests
crates/dioxus-storybook/tests/decorators.rs     render_decorated gained a StoryView
crates/dioxus-storybook-ui/tests/two_documents.rs  4 tests
examples/button-gallery/                        descriptions, story docs, ctx.view()
CLAUDE.md, log/index.log, docs/PLAN.md          M4 recorded
```

## Verified

- `cargo test --workspace` — 195 passing, green (was 173).
- `rust-analyzer diagnostics .` — clean (only `inactive-code`, as before).
- `cargo doc --no-deps --workspace` — clean, which it was not at session start.
- `cargo package -p dioxus-storybook-core` — packages and verifies.
- **In the browser**, `dx serve --platform web`:
  - `?id=forms-button--docs` renders the page: description from `story_meta!`,
    props table with real evaluated defaults, every story with its blurb, its
    live example and its source under "Show code";
  - `?id=data-display-badge--docs` shows "A small status pill." — the props-type
    fallback, with no `description:` anywhere;
  - the source block is the author's code, comments and indentation intact;
  - `&globals=theme:dark` themes every example on the page, because each is a
    real story inside the project decorator;
  - the viewport picker is gone on a docs page and back on a canvas;
  - the Canvas/Docs toggle sits at the right edge and **does not move** when the
    picker appears and disappears (F5); the story id is centred.

## Notes for the next session

M4 is complete. What it did *not* do, in the order it should probably be picked
up:

- **A sidebar entry per docs page.** The toggle is the only way in besides a
  URL. Storybook puts a "Docs" row as the first child of each component group,
  which means `TreeNode`/`RowKind`/`flatten` grow a variant and the keyboard
  navigation has to handle it. Deliberately skipped: the keyboard path is the
  one part of the shell no test can reach, and it has already produced two bugs.
- **Markdown in a description.** Today it is a plain string in a `<p>`. Rendering
  markdown means a dependency and a sanitiser decision. `docs/PLAN.md` already
  defers MDX; plain `.md` was the stated substitute and nothing was built for it.
- **Syntax highlighting** in the source block. It is a dark `<pre>` today. Doing
  it properly in wasm means `syntect` or a JS highlighter in the preview
  document; doing it cheaply means a Rust tokeniser in the shell.
- **`docs/plan-artifact.html`** is now four sessions behind `docs/PLAN.md`.

And carried, unchanged, from earlier milestones:

- **backgrounds** — still undecided whether it is a built-in or the documented
  decorator pattern. Do not write both.
- **measure / outline** — preview-side overlays; still the first addon that has
  to put something *inside* the frame.
- **`ErrorBoundary`** for a story returning `Err`, distinct from the panic hook.
- **The `Callback::new` per-render measurement**, carried since 0004. Note that
  a docs page now mounts every story of a component at once, so whatever that
  costs, a docs page pays it N times. That makes the measurement more
  interesting than it was.
