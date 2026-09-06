# Dioxus Storybook

A native [Storybook](https://storybook.js.org/) for Dioxus: a component
workbench where each component's states are declared as *stories*, rendered in
isolation, driven by controls auto-generated from the props type, and later
doubling as tests and documentation.

**Status:** M0 (spikes), M1 (walking skeleton), M2 (controls & actions), M3
(isolation, globals, environment) and M4 (docs) complete — the iframe split,
decorators, parameters, panic reporting, globals/toolbar, the viewport addon,
and a generated documentation page per component. M5 next.
**Scope:** a publishable open-source crate — semver + docs discipline apply.
**Name:** `dioxus-storybook` · MIT · © DW3Labs.
**Target:** Dioxus 0.7.10 · rustc 1.97.1 · dx 0.7.10 · web-first.

---

## ⚠️ Read the log first

**`log/index.log` is the entry point for every session.** Before doing anything
else, read it. It holds:

- a **HOT** section — what is planned next, what is blocked, and what *not* to do;
- a **SESSION LOG** table pointing at detailed entries in `log/NNNN-*.md`;
- **STANDING FACTS** — things that cost real time to learn. Do not re-derive them.

Several plausible-looking approaches in this project have already been tested and
**rejected with evidence** (see STANDING FACTS). Re-attempting them wastes a
session. When in doubt, grep the log before experimenting.

### Logging protocol

At the end of any session that changes the project's direction or knowledge:

1. Create `log/NNNN-short-slug.md`, following the shape of the existing entries:
   goal, what was done, findings (with evidence/commands), decisions, files
   produced, notes for the next session.
2. Add one row to the SESSION LOG table in `log/index.log`.
3. **Rewrite the HOT section wholesale** so it reflects reality at session end.
4. Promote anything expensive-to-relearn into STANDING FACTS.

Record negative results as carefully as positive ones — most of this project's
value so far is knowing which three approaches *don't* work.

---

## Layout

```
CLAUDE.md                  this file
LICENSE                    MIT, (c) DW3Labs
log/
  index.log                ← START HERE: summary, HOT next steps, standing facts
  0001-research-and-plan.md
  0002-m0-spikes.md
  0003-m1-walking-skeleton.md
  0004-m2-controls-and-actions.md
  0005-m3-isolation.md
  0006-m3-globals-and-toolbar.md
  0007-m3-viewport.md
  0008-m4-autodocs.md
docs/
  PLAN.md                  the working plan: features, problems, milestones
  M0-FINDINGS.md           full spike write-up
  plan-artifact.html       published version of the plan
crates/                    the published crates — all v0.1.0
  dioxus-storybook/        facade + prelude (re-exports dioxus::prelude)
  dioxus-storybook-core/   StoryDef, Meta, args, Registry, Channel, url state
  dioxus-storybook-macro/  #[story], story_meta!, Controls, ControlEnum
  dioxus-storybook-build/  build-script story indexer
  dioxus-storybook-ui/     manager shell + preview harness
examples/
  button-gallery/          5 components, 17 stories — the thing you actually run
spikes/                    evidence, kept but superseded by crates/
  s1-hotreload/            vertical slice app (EXCLUDED from the workspace)
  s2-registry/             story-registration mechanism comparison
  s3-controls/             #[derive(Controls)] proc macro + 15 tests
  s4-iframe/               M3: one bundle, loaded twice, talking to itself
Cargo.toml                 workspace (edition 2024)
```

## Commands

```bash
# Run the storybook. This is the main loop.
cd examples/button-gallery && dx serve --platform web

# The whole suite: 202 tests + doctests. Everything must stay green.
cargo test --workspace

# Definition of done for anything macro-facing. Must be clean.
rust-analyzer diagnostics .

# Also definition of done, since M4: docs.rs would fail on a broken link, and
# nothing else in the loop runs rustdoc. It was already red at M4's start.
cargo doc --no-deps --workspace

# Check the publish metadata still holds
cargo package -p dioxus-storybook-core

# --- M0 archaeology, rarely needed ---
cd spikes/s1-hotreload && dx serve --platform web   # note the cd, it is load-bearing
cargo test -p s3-test                               # 15 tests, real Dioxus types
./spikes/s2-registry/verify.sh                      # the registration matrix
cd spikes/s4-iframe && dx serve --platform web      # the M3 iframe spike
```

---

## Architecture decisions already made

| | Decision | Why |
|---|---|---|
| Registry | **build-script codegen** | `linkme` and `inventory` both fail on wasm — see log |
| Controls | **`#[derive(Controls)]`** on the Props struct | macro sees names, types and doc comments statically, and the applier it emits is compiler-checked |
| Args → props | an **applier**, not a deserializer | props hold `EventHandler`/`Element`, which aren't serializable |
| Isolation | **`Channel` trait from commit 1**; iframe + `postMessage` since M3 | the transport changed without a single addon changing |
| M3 split | **one bundle, loaded twice** — `?viewMode=preview` — not two bundles | isolation is a property of the *document*; a second build unit costs the shared registry and buys nothing |
| Wire format | **hand-rolled, length-prefixed**, no `serde` | nine tags over a closed vocabulary, and `ArgMap` already had a tested textual codec |
| Handshake | preview emits `PreviewReady`; manager **resyncs** rather than queues | every manager→preview message is state, not a command |
| Panics | a **panic hook** posts `PreviewPanicked`; there is no recovery | `panic = "abort"` means the module is gone, not unwound |
| Decorators | `fn(&StoryContext, Element) -> Element` at three levels | a context struct now, because widening the signature later would break every decorator |
| Parameters | **`ResolvedParameters`** consults three levels in order | no allocation, stays `Copy`, innermost wins |
| Globals | **args machinery reused** — a `Control` declaration, an `ArgMap` of values | widget, URL encoding and wire encoding all came free; there must not be a fourth value vocabulary |
| Globals on the wire | **only the selections**, never the resolved set | both halves share the `&'static` declarations, and an empty map then means "nothing changed" |
| Viewport | **a style on the iframe** — list on the `Project`, choice a parameter, selection a global | the shell owns the frame's width, so nothing crosses the wire; and `ParamValue` cannot hold a record, so the list is a declaration |
| Autodocs | **an entry id, `title--docs`**, in the same space as a story | no `viewMode=docs`, no new wire message, no second transport; a real story wins the collision |
| Docs page location | **the preview document** | its examples are real stories, and a story needs a scope, its decorators and its own CSS cascade |
| Story source | **`Span::source_text()`**, not the token stream | a token stream renders `..base ()`; the span gives the code as written, comments included |
| Doc comments quoted | **summary paragraph only**, inline markup rendered | rustdoc's own short-description rule; three constructs (code, emphasis, links) map to elements |
| Platform | **web-first** | the static build is both the shareable artifact and the screenshot-test substrate |
| Story bodies | **fn pointers invoked in-scope** | `rsx!`/`EventHandler::new` need an active Dioxus scope |
| Ambition (D1) | **publishable crate**, M0..M6 | drives semver discipline: private fields + `const` builders, `#[non_exhaustive]`, `#![deny(missing_docs)]` |
| Namespace (D5) | **`dioxus-storybook-*`**, MIT, © DW3Labs | discoverability beat `dx-story`'s trademark distance |
| Story ↔ component | **`macro_rules!` bridge from `story_meta!`** | `#[story]` knows the props type, `story_meta!` knows the component; neither can name the other's half |
| Actions | **substitution in `wire_actions`**, emitted by the same derive | an `EventHandler` is observed, not edited, so `apply` cannot do it |
| Action payloads | **autoref specialisation on `Debug`** | prints what it can without putting a bound on the user's prop types |
| Default arg values | **the preview computes them, the manager receives them** | evaluating props needs a scope, and the manager renders no stories |
| Story scope | **one remounted `StoryHost` per story** | a story's hooks must not share the preview's hook list |

## Gotchas that will bite you

- **`dx serve --hot-patch` does not build** on wasm (`exceptions proposal not
  enabled`). Use plain `dx serve --platform web`. `panic = "abort"` does *not*
  fix it — already tried.
- **`dx` and the `dioxus` crate versions must match exactly** or dx refuses to run.
- **`dx serve` does not notice changes under `crates/`,** nor a brand-new story
  file. Restart it (~20-25s) rather than wondering why nothing changed.
- **`use_effect` does not run under a bare `VirtualDom`.** `process_events()`
  does, but only with no dirty scopes. This is why the preview's `PreviewReady`
  handshake is a hook, not an effect.
- **`window.parent` is `window` when a page is not framed,** and
  `window.onmessage` is shared with browser extensions. Both are why every
  message carries a magic prefix and its sender's role.
- **`spikes/s1-hotreload` is excluded from the workspace.** `cd` into it;
  `--package s1-hotreload` from the root will not find it.
- **Workspace is edition 2024** → `#[unsafe(no_mangle)]`, not `#[no_mangle]`.
- **rsx! format strings interpolate idents only.** `{a * b}` fails to parse;
  compute into a `let` first.
- **Any file that invokes `rsx!` needs `dioxus::prelude::*` in scope** — it
  expands to unqualified `dioxus_core::`/`dioxus_signals::` paths. This includes
  stories files, where our bridge expands `rsx!` for you; that is why
  `dioxus_storybook::prelude` re-exports the dioxus prelude.
- **The scope rule is broader than rendering.** Anything that evaluates a
  story's props needs an active scope, `StoryDef::base_args()` included.
- **Writing a `Signal` inside an `Fn` callback:** copy it *inside* the body.
  Capturing a `mut` copy makes the closure `FnMut`.
- **Do not construct `StoryDef`/`Meta` with struct literals.** Fields are private
  so that adding one stays non-breaking; use the `const` builders.
- **A `<select>` in the toolbar fights the sidebar's keyboard navigation.** The
  shell listens for arrows/Enter/`/` on the whole `.dxsb` element. Toolbar
  controls stop propagation (`swallow_toolbar_keys`); the search input still
  does not. No test can catch this — a headless `VirtualDom` cannot deliver a
  keystroke.
- **A derived `Default` on a config struct is a trap** once its `const fn new()`
  stops being all-zeroes. `Project::new()` fills in `DEFAULT_VIEWPORTS`, so
  `Default` is written out by hand to match it.
- **`100vh` in a decorator is right on the canvas and wrong on a docs page.**
  The canvas is one story owning the document; a docs page stacks a dozen
  examples. There is no CSS fix — `vh` is always the viewport — so the decorator
  must branch on `ctx.view()`. See `StoryView`.
- **`key` only works inside a list.** Dioxus consults it in `diff_keyed_children`
  only; a lone keyed child is diffed in place and never remounts. The preview
  wraps `StoryHost` in `for def in [def]` for exactly this reason — do not
  "simplify" it away.
- **Editing a proc macro's output needs a rust-analyzer restart.** The editor
  keeps the old dylib while checking against the new trait, so you get a false
  `E0046` at every derive site. `cargo check --workspace --all-targets` clean +
  `rust-analyzer diagnostics .` clean means it is staleness, not your code.
- **`try_consume_context` panics with no runtime,** not just with no context.
  Guard with `Runtime::try_current()` anywhere that has to be total.
- **Never emit a `macro_rules!` from one proc macro for another proc macro's
  expansion to call with a value argument.** rustc accepts it; rust-analyzer
  reports a false `E0425` at every call site (rust-analyzer#10644, open since
  2021). Emit a plain `fn` instead. `rust-analyzer diagnostics .` must stay
  clean — a green `cargo test` does not prove the authoring surface is usable.
- **Never script an edit with macOS `sed -i ''`** when measuring hot reload — it
  writes a sibling temp file that dx's watcher picks up instead of your file.
- **LTO const-folds static registries into literals.** A registry test that only
  counts entries can pass while proving nothing. Use `black_box` and read real
  bytes out of linear memory.

## Working style for this project

- Spikes over speculation: when a design question has a testable answer, write
  the throwaway binary and run it. Two of M0's three findings contradicted
  confident prior assumptions.
- Verify by execution, not compilation. "It builds" is not "it works" — the
  `inventory` spike compiled cleanly and was still wrong.
- When a measurement looks too good, suspect the measurement first.
