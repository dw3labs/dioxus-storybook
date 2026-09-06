# 0003 — M1: the walking skeleton

**Date:** 2026-09-06 · **Status:** complete
**Toolchain:** rustc 1.97.1 · dx 0.7.10 · dioxus 0.7.10 · macOS aarch64

M1 as scoped in `docs/PLAN.md`: browse and render stories, with the selected
story and its args carried in the URL. Two blocking decisions were answered by
the human first.

---

## Decisions taken

| # | Decision | Answer |
|---|---|---|
| D1 | How far does this go? | **(b) publishable open-source crate** → M0..M6, semver + docs discipline |
| D5 | Crate namespace | **`dioxus-storybook-*`** (all five candidate names were free on crates.io) |
| — | License | MIT, © DW3Labs |

D1 is why M1 looks the way it does: private fields with `const` builders instead
of public struct literals, `#[non_exhaustive]` on the enums that will grow,
`#![deny(missing_docs)]` everywhere, and per-crate README + LICENSE.

`dioxus-storybook` was chosen over `dx-story` for discoverability, accepting the
"Storybook" trademark proximity. Worth revisiting only if Chromatic objects.

---

## What was built

Five published crates plus a worked example.

```
crates/dioxus-storybook          facade + prelude (re-exports dioxus::prelude)
crates/dioxus-storybook-core     StoryDef, Meta, args, Registry, Channel, url
crates/dioxus-storybook-macro    #[story], story_meta!, Controls, ControlEnum
crates/dioxus-storybook-build    build-script indexer
crates/dioxus-storybook-ui       manager shell + preview harness
examples/button-gallery          2 components, 9 stories, 2 sidebar groups
```

M1's checklist from the plan, item by item:

- **Core types** — `StoryDef` holds fn pointers, per LOG-0002/S3.
- **Codegen registry** — `dioxus-storybook-build` scans for `#[story]` and emits
  explicit `&'static` references. It also *declares* the story modules via
  `#[path]`, so there is no `mod.rs` to maintain: dropping a file into
  `src/stories/` is enough.
- **`#[story]` + `story_meta!`** — both forms work: return the props type and get
  the component rendered for you, or return `Element` for full control.
- **In-process `Channel`** — the manager never calls the preview directly.
- **Manager shell** — sidebar tree from `title`, fuzzy search, keyboard nav
  (↑/↓ browse, ←/→ fold, Enter, `/` to search, Esc to clear).
- **Preview harness** — subscribes to the channel; resolves its own landing
  story from the URL exactly as an iframe would.
- **URL state** — `?id=…&args=k:v;k:!true`, both directions.

Deliberately **not** in M1: the controls *panel*. `#[derive(Controls)]` produces
the props table and the applier already (both tested), but the widgets that edit
them are M2 as planned. Args still flow, via the URL.

## The macro bridge, and the rust-analyzer trap

`#[story]` knows the props type (the function's return type) but not the
component. `story_meta!` knows the component but not the props type. They have to
meet somehow.

**The first version used a `macro_rules!` bridge** emitted by `story_meta!`:

```rust
macro_rules! __dxsb_render_component {
    ($props:expr) => { rsx! { Button { ..$props } } };
}
```

It compiled, every test passed, and wasm built — **and it made rust-analyzer
report a false `E0425: no such value in this scope` on every props-form story.**
Found only because the human opened the example in an editor. `cargo test` can
never catch this class of bug.

### Isolating it

Four experiments, each one `rust-analyzer diagnostics` run:

| # | Change | Result |
|---|---|---|
| 1 | Hand-write the `__DXSB_META` const instead of generating it | still fails — not a proc-macro-to-proc-macro problem |
| 2 | `self::__DXSB_META`, then `__DXSB_META` spanned at the user's fn ident | still fails — not fixable by path or span |
| 3 | Drop the meta reference, keep the bridge | still fails — the meta was never the culprit |
| 4 | Keep the meta reference, drop the bridge | **clean** |

Two facts fell out, and the second is the useful one:

- Referencing a plain module **item** (a `const`, a `fn`) created by one
  proc-macro expansion from inside another expansion is **fine** in RA.
- Passing a **value** created in an attribute expansion as an argument into a
  proc-macro-generated `macro_rules!` is **not**. RA cannot resolve it, rustc can.

Closest upstream issue: [rust-analyzer#10644](https://github.com/rust-lang/rust-analyzer/issues/10644),
"Inconsistent unresolved-macro-call errors for macro_rules macro generated via
proc-macro" — open since 2021, labelled `A-macro`. Not something to wait for.

Note this is *not* the common "proc macro library has no proc macros" error,
which means the dylib failed to load. Ours loaded fine: the derives expanded,
`story_meta!` expanded, and only the bridge failed.

### The fix

`story_meta!` now emits a **plain function** instead of a macro:

```rust
pub fn __dxsb_render_component(props: ButtonProps) -> Element {
    rsx! { Button { ..props } }
}
```

which costs it knowing the props type. It infers `{Component}Props` — the Dioxus
`#[component]` convention — and `props: MyProps` overrides when that is wrong.
`rust-analyzer diagnostics` over the whole workspace is now clean.

Alternatives considered and rejected: calling the component function directly as
`Button(props)`, which compiles but runs the component's hooks in the *parent's*
scope instead of giving it one of its own; and folding both macros into a single
`stories! { ... }` block, which removes the coupling but puts every story body
inside a macro, degrading error spans and rustfmt on the code that gets edited
most.

## Verification

`cargo test --workspace` — 43 tests + 8 doctests, all passing. Per the project's
working style these execute rather than merely compile:

- **`crates/dioxus-storybook/tests/end_to_end.rs`** (10) renders stories through
  a real `VirtualDom` and asserts on SSR output: overrides applied, unset args
  falling back to typed defaults, garbage args degrading rather than panicking,
  doc comments reaching `ArgType`.
- **`crates/dioxus-storybook-ui/tests/shell.rs`** (7) renders the whole manager.
  The browser layer is a no-op off wasm32, so the shell asserts on the host.
- **`core/tests/`** (20) cover tree building, collapse flattening, search
  ranking, URL round-trips including hostile input, and channel semantics —
  including a listener that emits during its own dispatch, which the preview
  actually does.
- **`build/src/scan.rs`** (6) prove `#[story]` inside a comment, a string, a raw
  string or a doc comment does not register a story.
- **`dx build --platform web`** succeeds: 30s cold, ~4s warm.
- **`cargo package -p dioxus-storybook-core`** succeeds.
- **`rust-analyzer diagnostics .`** is clean across the workspace. This is now
  part of the definition of done — see the finding below for why `cargo test`
  passing is not sufficient evidence that the authoring surface works.

---

## Findings

### 1. `rsx!` needs `dioxus::prelude::*` at the call site

`rsx!` expands to unqualified `dioxus_core::` and `dioxus_signals::` paths, which
only resolve because `dioxus::prelude` re-exports those crate names. Since the
`story_meta!` bridge expands `rsx!` *inside the user's stories file*, that file
must have the dioxus prelude in scope. Caught by `badge.rs`, which imported only
our prelude and failed with `unresolved import dioxus_signals`.

Fixed by having `dioxus_storybook::prelude` re-export `dioxus::prelude::*`. Two
globs re-exporting the same items do not conflict, so users who import both are
fine.

### 2. `base_args()` needs a scope, not just `render()`

LOG-0002 recorded that `rsx!` and `EventHandler::new` need an active scope. What
was not recorded is that this applies to **anything that evaluates a story's
props** — `base_args()` calls the story function, whose props may build an
`EventHandler`. A test calling `PRIMARY.base_args()` outside a scope panicked.
Now documented on the method.

### 3. rust-analyzer is a separate test surface

See the bridge section above. A proc-macro API can be correct under rustc and
unusable in an editor, and for a crate whose whole product is authoring
ergonomics that is a shipping blocker, not a cosmetic issue. `rust-analyzer
diagnostics .` runs headless and belongs in CI.

Two smaller RA false positives in `channel.rs` were also worked around, both by
writing types out rather than inferring them: `Weak::upgrade` into a
`Rc<Inner>` binding, and the tuple in a `retain` closure. Both produced
`E0282: type annotations needed` in RA and compiled fine under rustc.

### 4. Signals in an `Fn` callback must be copied inside the body

`Channel::subscribe` takes `Rc<dyn Fn(&Event)>`. Writing to a captured `Signal`
needs `&mut`, which makes the closure `FnMut` — even though `Signal` is `Copy`,
because the *binding* is being mutated. Taking a fresh copy inside the body
instead keeps it `Fn`:

```rust
Rc::new(move |event: &Event| {
    let mut current = current;   // Copy, per call
    ...
})
```

### 5. Generated story statics must be `pub`

The generated registry module is an *ancestor* of the story file's module, and
Rust privacy runs downward only. `#[story]` therefore always emits `pub static`
regardless of the story function's own visibility.

### 6. dx builds a workspace member fine

`spikes/s1-hotreload` had to be excluded from the workspace, which suggested dx
disliked workspaces. It does not — `examples/button-gallery` is a member and
builds. What did have to move was `[profile.wasm-dev] panic = "abort"`, which
Cargo only honours in the workspace root manifest.

---

## Files produced

```
Cargo.toml                                  workspace: crates/*, examples/*, shared package metadata
LICENSE                                     MIT, (c) DW3Labs
crates/dioxus-storybook{,-core,-macro,-build,-ui}/
examples/button-gallery/
```

## Notes for the next session

- **M2 is the differentiating milestone.** The controls panel is mostly a port:
  `spikes/s1-hotreload/src/main.rs` already has working widgets for every
  `Control` variant. What is new is `use_args` write-back and wiring
  `EventHandler` props to an actions panel.
- **Before the first publish:** the workspace has no git repo, so no
  `repository`/`homepage`/`documentation` in the manifests — `cargo package`
  warns about it. Set those up before `cargo publish`.
- **Publish order** if publishing now: core → macro → build → ui → facade.
- **Still open:** D3 (ship M5 publish early, or M6 testing early?) and D4
  (desktop mode in scope at all?). Neither blocks M2.
