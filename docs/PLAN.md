# Dioxus Storybook — Research & Project Plan

> Status: M0 + M1 complete · Date: 2026-09-06 · Target: Dioxus 0.7.10
> Shipping as **`dioxus-storybook`**, MIT, © DW3Labs — a publishable crate (D1b).
>
> **Superseding results live in `docs/M0-FINDINGS.md` and `log/0002-m0-spikes.md`.**
> Sections marked ✅ RESOLVED below were settled by running code.

---

## Part 1 — How Storybook actually works

Storybook is not "a component gallery". It is a **build pipeline + a two-process runtime + an event bus**, and every visible feature is an addon plugged into that bus. Understanding those three layers is what lets us decide what to copy and what to reinvent.

### 1.1 The authoring format: CSF (Component Story Format)

An ES module with one default export (`meta`) and N named exports (stories):

```js
// Button.stories.ts
const meta = {
  title: 'Forms/Button',      // sidebar path (optional — inferred from file path)
  component: Button,
  args:       { label: 'Hi' },// default prop values
  argTypes:   { variant: { control: 'select', options: [...] } },
  parameters: { backgrounds: { default: 'dark' } },
  decorators: [withTheme],
  tags:       ['autodocs'],
};
export default meta;

export const Primary = { args: { variant: 'primary' } };
export const Loading = { args: { loading: true }, play: async ({canvas, userEvent}) => {...} };
```

Six orthogonal concepts, and they are the whole data model:

| Concept | What it is | Merge order |
|---|---|---|
| **args** | Serializable prop values. Change one → component re-renders. | global → component → story → URL |
| **argTypes** | Metadata *about* each arg: type, description, default, which control widget. Auto-inferred, hand-overridable. | same |
| **parameters** | Static, non-serializable config consumed by addons (viewport list, backgrounds, docs opts). | same |
| **decorators** | Wrapper functions — theme provider, router, padding. | global → component → story (outermost first) |
| **globals** | App-wide state settable from the toolbar (theme, locale). Read by decorators. | single scope |
| **tags** | Strings that opt a story into behaviours (`autodocs`, `!test`, `dev`). | inherited + overridable |

### 1.2 The build pipeline

1. **Indexer** — globs `**/*.stories.*` at build time, produces `index.json`: a flat list of `{id, title, name, importPath, tags}`. The sidebar tree is derived from `title` split on `/`.
2. **Docgen** — a compiler plugin (`react-docgen` / TS compiler API) reads the component's prop types and JSDoc and emits argTypes automatically. **This is the magic that makes controls "just work".**
3. **Builder** (Vite or Webpack) emits *two* bundles:
   - **Manager** — the Storybook UI shell (sidebar, toolbar, addon panels). Pure React, framework-agnostic, ships prebuilt.
   - **Preview** — `iframe.html`, containing the user's framework runtime + their stories.

### 1.3 The runtime: two processes, one channel

```
┌─────────────────── Manager (parent window) ────────────────────┐
│  Sidebar    │  Toolbar (globals)                               │
│  ─────────  ├──────────────────────────────────────────────────┤
│  Forms/     │  ┌──── <iframe> Preview ──────────────────────┐  │
│   Button    │  │  user's framework + component renders here │  │
│    Primary  │  └────────────────────────────────────────────┘  │
│    Loading  ├──────────────────────────────────────────────────┤
│             │  Addon panels: Controls │ Actions │ A11y │ Tests │
└─────────────┴──────────────────────────────────────────────────┘
         ▲                                              ▲
         └──────── postMessage channel (event bus) ─────┘
    setCurrentStory · updateStoryArgs · storyRendered · storyErrored
    logAction · a11yResult · playFunctionThrew · ...
```

The **iframe is load-bearing**, not cosmetic:
- CSS from the manager cannot leak into your component, and vice versa.
- A component that throws or infinite-loops cannot take down the UI.
- The preview URL (`iframe.html?id=forms-button--primary&args=label:Hey`) is independently addressable — which is exactly how the test runner, visual-regression snapshotting, and embedding all work.

Every addon is a pair: a **manager-side** half (renders a panel/toolbar item) and a **preview-side** half (a decorator or instrumentation), talking over the channel. There is no privileged core feature — Controls, Actions, and A11y are all just addons.

### 1.4 Story lifecycle on selection

```
user clicks story → manager emits setCurrentStory(id)
  → preview loads story → applies globals, parameters
  → composes decorators (global ∘ component ∘ story)
  → renders with merged args
  → emits storyRendered → manager unblocks panels
  → if story.play exists: run it, instrument every interaction, stream steps to Interactions panel
```

---

## Part 2 — Feature inventory: what makes Storybook great

Grouped by the job it does. `Value` = how much it matters. `Cost` = difficulty to build for Dioxus.

### A. Isolation & iteration (the core loop)

| # | Feature | Why it matters | Value | Cost |
|---|---|---|---|---|
| A1 | Render a component **outside the app**, no routing/auth/data setup | The single reason Storybook exists. Cuts the edit→see cycle from minutes to seconds. | ★★★★★ | Low |
| A2 | **Hot reload** preserving story selection & args | Without it the tool is a slideshow, not a workbench. | ★★★★★ | Low (Subsecond) |
| A3 | **Sidebar tree** from `title` paths + fuzzy search + keyboard nav | Navigating 300 stories. | ★★★★★ | Low |
| A4 | **Deep-linkable URLs** encoding story id + args | "Here's the exact broken state" in a Slack message. | ★★★★☆ | Low |
| A5 | **Isolated iframe** — CSS and crashes contained | Everything downstream (tests, snapshots, embeds) depends on it. | ★★★★★ | Medium |
| A6 | **Zero-config startup** (`storybook init` detects your stack) | Adoption cliff if setup is a chore. | ★★★★☆ | Medium |

### B. Controls & interactivity

| # | Feature | Why it matters | Value | Cost |
|---|---|---|---|---|
| B1 | **Auto-generated controls** from prop types — no config | The "wow" moment. Explore the whole prop space without writing a story per permutation. | ★★★★★ | **High** |
| B2 | Control widget per type: text, number, range, bool, select, radio, color, date, object, file | Depth of exploration. | ★★★★☆ | Medium |
| B3 | **Actions** — event handlers auto-logged to a panel | See what a component *emits*, not just how it looks. | ★★★★★ | Medium |
| B4 | **Globals + toolbar** (theme, locale, direction) | Test a component in dark mode / RTL in one click. | ★★★★☆ | Low |
| B5 | **Viewport** switcher — responsive breakpoints | Responsive verification without resizing a window. | ★★★★☆ | Low |
| B6 | **Backgrounds** switcher | Verify on light/dark/brand surfaces. | ★★★☆☆ | Low |
| B7 | **Measure & Outline** — box-model overlay, layout debugging | Catches spacing bugs fast. | ★★★☆☆ | Low |
| B8 | **Highlight** — programmatically outline elements | Powers a11y violation pinpointing. | ★★☆☆☆ | Low |
| B9 | `useArgs` — story writes back to args (controlled inputs work) | Toggles/inputs feel real instead of frozen. | ★★★★☆ | Medium |

### C. Composition & reuse

| # | Feature | Why it matters | Value | Cost |
|---|---|---|---|---|
| C1 | **Decorators** at 3 levels | Wrap in theme/router/store once, not per story. | ★★★★★ | Low |
| C2 | **Parameters** with 3-level merge | Uniform addon config surface. | ★★★★☆ | Low |
| C3 | **Story composition** — one story reuses another's args | DRY variants. | ★★★☆☆ | Low |
| C4 | **Portable stories** — import a story into a unit test | Stories become the single source of fixture truth. | ★★★★☆ | Medium |
| C5 | **Loaders / beforeEach / mount** — async setup, mocked data | Stories for data-driven components. | ★★★☆☆ | Medium |

### D. Documentation

| # | Feature | Why it matters | Value | Cost |
|---|---|---|---|---|
| D1 | **Autodocs** — a docs page generated per component, free | Docs that can't rot, because they're the running code. | ★★★★★ | Medium |
| D2 | **Props table** from docgen + doc comments | The reference every consumer wants. | ★★★★★ | Medium |
| D3 | **Source snippet** per story, copyable | "How do I get this?" answered inline. | ★★★★☆ | Medium |
| D4 | **MDX** — hand-written prose + embedded live stories | Design-system guidelines pages. | ★★★★☆ | High |
| D5 | **Doc blocks** (`<Canvas>`, `<ArgsTable>`, `<Story>`) as composable primitives | Custom doc layouts. | ★★★☆☆ | Medium |

### E. Testing

| # | Feature | Why it matters | Value | Cost |
|---|---|---|---|---|
| E1 | **Render/smoke tests** — every story is a free test | Enormous coverage for zero extra authoring. | ★★★★★ | Low |
| E2 | **Play functions** — scripted interaction + assertions | Behaviour tests colocated with the visual case. | ★★★★★ | **High** |
| E3 | **Interactions panel** — step-through debugger with time travel | Makes failing tests diagnosable, not just red. | ★★★★☆ | High |
| E4 | **A11y (axe) checks** per story, with violation highlighting | Catches contrast/ARIA/keyboard issues at authoring time. | ★★★★★ | **Low** (reuse axe-core) |
| E5 | **Visual regression** — screenshot diffing (Chromatic) | Catches unintended visual change across a design system. | ★★★★★ | Medium |
| E6 | **CI integration** — headless run of all of the above | Turns the tool into a gate, not a toy. | ★★★★☆ | Medium |
| E7 | **Coverage** reporting | Gap-finding. | ★★☆☆☆ | Medium |

### F. Collaboration & platform

| # | Feature | Why it matters | Value | Cost |
|---|---|---|---|---|
| F1 | **Static build** → deployable site | Share with design/PM/other teams. Free artifact. | ★★★★★ | Low |
| F2 | **Addon API** — panels, toolbars, tabs, presets, channel | The ecosystem. Storybook won because it was extensible. | ★★★★★ | Medium |
| F3 | **Composition (refs)** — merge multiple Storybooks into one sidebar | Monorepo / multi-package design systems. | ★★★☆☆ | Medium |
| F4 | **Theming** the Storybook UI itself | Brandable design-system home. | ★★☆☆☆ | Low |
| F5 | Design-tool integrations (Figma embeds) | Design/code parity. | ★★★☆☆ | Low |

**The short version of "why Storybook is great":** *auto-generated controls from types* (B1) + *stories double as tests* (E1/E2) + *docs generated from the same source* (D1) + *an extensible channel so everything else is an addon* (F2). Everything else is polish on top of those four.

---

## Part 3 — The Dioxus translation problem

Six things in the JS design do not survive the port. These are the real engineering risks.

### P1 — No runtime module discovery
JS does `import.meta.glob('**/*.stories.js')`. Rust links everything at compile time.

**Options**
- **(a) Link-section registry** — `linkme::distributed_slice`. A `#[story]` attribute macro pushes a `&'static StoryDef` into a slice; the binary collects it at startup. Zero codegen, best ergonomics. **Risk:** wasm32 + `--gc-sections` can drop the section; needs a spike.
- **(b) Build-script codegen** — CLI/`build.rs` walks `src/**/*.stories.rs`, emits a `stories.rs` registry `include!`d into the harness. Boring, deterministic, always works.
- **(c) Explicit registration** — user writes `registry.add::<Primary>()`. Zero magic, worst ergonomics.

**✅ RESOLVED IN M0 — option (b), codegen.** `linkme` does not compile for
wasm32 at all; `inventory` compiles but silently drops any story in an
unreferenced codegen unit (passes at `codegen-units=1`, fails at 16 and 256),
so stories vanish in dev builds and reappear in release. Build-script codegen
passes in both profiles. Do not revisit (a) or (c) without reading LOG-0002.

### P2 — No docgen / no reflection
There is no `react-docgen` for Rust. But we have something better: **we own the derive macro.**

```rust
#[derive(Props, PartialEq, Clone, Controls)]
pub struct ButtonProps {
    /// Text shown inside the button.        ← becomes the docs description
    pub label: String,                       ← control: text
    pub variant: ButtonVariant,              ← control: select (from #[derive(ControlEnum)])
    #[control(range(min = 0.0, max = 4.0, step = 0.25))]
    pub scale: f32,                          ← control: range
    pub disabled: bool,                      ← control: toggle
    #[control(skip)]
    pub children: Element,
    pub onclick: EventHandler<MouseEvent>,   ← auto-wired into the Actions panel
}
```

The proc macro sees the field names, types, and doc comments — everything docgen extracts, available statically and *type-checked*. This is a genuine advantage over the JS original.

### P3 — Dynamic args → typed props
Storybook spreads a JSON object onto props. Rust needs the inverse. Serde won't work: `Props` contain `EventHandler` and `Element`, which aren't serializable.

**Solution:** the `Controls` derive generates an *applier*, not a deserializer:
```rust
impl Controllable for ButtonProps {
    fn arg_types() -> &'static [ArgType] { ... }
    fn apply(&self, args: &ArgMap) -> Self  // overlay dynamic values onto a typed base
}
```
Only fields whose type implements `ArgValue` participate; the rest come from the story's base props. Non-controllable fields simply don't get a widget — same as Storybook's behaviour for functions.

### P4 — Manager/preview isolation
Two Dioxus web apps + `postMessage` is the faithful port, but it means two wasm bundles and a build-orchestration story on day one.

**Decision:** define `trait Channel { fn send(&self, Msg); fn subscribe(&self, f); }` from the very first commit. Implement it **in-process** (single bundle, preview in a container div) for M1–M2, then swap in the `postMessage` iframe implementation in M3 **without rewriting a single addon**. The abstraction is cheap; the rewrite is not.

### P5 — Interaction testing has no `@testing-library`
`play` functions need DOM querying and user-event simulation. Nothing exists for Dioxus.

**Solution:** a small `dx-story-testing` crate over `web-sys`: `get_by_role`, `get_by_text`, `get_by_test_id`, and `user_event::{click, type_text, hover, keyboard}` with proper event sequencing. Bounded but real work — hence a late milestone.

### P6 — Multiplatform ambiguity
Dioxus targets web, desktop, and mobile. Storybook is web-only.

**Decision: web-first.** The static build is the shareable artifact and the substrate for screenshot testing. Desktop mode (`dx serve --platform desktop`) is nearly free later since it's the same code in a webview — but it is not the target.

### The load-bearing assumption — ✅ CORRECTED IN M0

The original claim was that the project rests on **Subsecond** hot-patching Rust
while preserving app state. That is **wrong**. `dx serve --hot-patch` does not
build on wasm at all (`exceptions proposal not enabled`; `panic = "abort"` does
not fix it).

What the project actually rests on is the **incremental rebuild being ~1.5s** —
measured, with hot-patching switched off entirely. rsx-only edits hot-reload in
0.36s. Subsecond becomes an optimisation to adopt if it is fixed upstream, not a
precondition.

**Consequence:** a rebuild reloads the page and destroys every in-memory signal,
so **A4 (URL-encoded story id + args) moves from M2 into M1** — it is what makes
the dev loop survive an edit.

---

## Part 4 — Proposed architecture

```
dioxus-storybook          facade + prelude                                    [M1 ✅]
dioxus-storybook-core     StoryDef/Meta/ArgType/ArgValue/Parameters · registry · channel  [M1 ✅]
dioxus-storybook-macro    #[story] · story_meta! · Controls · ControlEnum      [M1 ✅]
dioxus-storybook-build    build-script story indexer                           [M1 ✅]
dioxus-storybook-ui       manager shell + preview harness                      [M1 ✅]
dioxus-storybook-addons   controls · actions · viewport · backgrounds · a11y · measure · outline
dioxus-storybook-testing  DOM queries + user-event for play functions
dioxus-storybook-cli      `dioxus-storybook dev|build|test|snapshot`
```

**Authoring surface (target ergonomics):**

```rust
// src/components/button.stories.rs
use dioxus::prelude::*;
use dx_story::prelude::*;
use super::{Button, ButtonProps, ButtonVariant};

story_meta! {
    title: "Forms/Button",
    component: Button,
    args: ButtonProps { label: "Click me".into(), ..Default::default() },
    tags: [autodocs],
}

#[story]
fn primary() -> ButtonProps {
    ButtonProps { variant: ButtonVariant::Primary, ..meta_args() }
}

#[story(name = "With Icon")]
fn with_icon(args: Args) -> Element {          // escape hatch: full control of rsx
    rsx! { Button { ..args.into(), Icon { name: "save" } "Save" } }
}

#[story]
#[play(async |c: Canvas| {
    c.get_by_role(Role::Button).click().await;
    assert!(c.get_by_text("Saved!").is_present());
})]
fn submits() -> ButtonProps { ... }
```

---

## Part 5 — Milestone plan

### M0 · Feasibility spikes — ✅ **COMPLETE, verdict GO**
Three go/no-go questions. Each is a throwaway binary.
1. **Subsecond + story registry.** Does `dx serve` hot-patch survive a story-registry lookup? Does editing a `.stories.rs` file update the running preview while preserving the selected story?
2. **`linkme` on wasm32.** Does a distributed slice survive `wasm-opt` and `--gc-sections` under `dx build --release`? If no → codegen fallback (P1b) becomes the plan of record.
3. **Props introspection.** Prototype `#[derive(Controls)]` on a 6-field struct with an enum, an `Option`, and an `EventHandler`. Confirm doc comments reach the macro and the applier type-checks.

**Result:** met, with hot-patching off — a story renders, controls edit props
live, and a source edit rebuilds in 1.5s. S3 passed 15/15 against real Dioxus
types. S2 killed both automatic-registration options. Details in
`docs/M0-FINDINGS.md`.

### M1 · Walking skeleton — ✅ **COMPLETE**
`dx-story-core` types (`StoryDef` holding **fn pointers**, invoked in-scope — `rsx!`/`EventHandler::new` need an active Dioxus scope) · `#[story]`/`story_meta!` macros · **codegen registry** · in-process `Channel` · manager shell (sidebar tree from `title`, search, keyboard nav) · preview harness · **URL state: story id AND args (moved up from M2 — this is what survives a rebuild)**.
> **Delivers:** browse and render stories. Already useful.
>
> **Shipped** as five crates under `crates/` plus `examples/button-gallery`;
> 43 tests + 8 doctests. The controls *panel* stayed in M2 as planned, but
> `#[derive(Controls)]` and the applier are in and tested. One design change:
> `story_meta!` bridges to the component with a generated **function**, not a
> `macro_rules!`, because the macro version broke rust-analyzer. See LOG-0003.

### M2 · Controls & actions — ✅ **COMPLETE**
`#[derive(Controls)]` + `ControlEnum` · `ArgValue` for primitives, enums, `Option<T>`, `Vec<T>` · widgets: text, number, range, bool, select, radio, color · Controls panel with reset-to-default · (args-in-URL moved to M1) · `EventHandler` auto-wiring → Actions panel · `use_args` write-back.
> **Delivers:** the "wow" loop. This is where it stops being a gallery.
>
> **Shipped.** 67 tests + 10 doctests. Two findings changed the design: the
> manager cannot compute a story's default args (it needs a scope, and at M3 the
> story fns are in the other bundle), so the preview publishes them as
> `StoryPrepared`; and story bodies needed a scope of their own, because their
> hooks were landing in the preview's hook list. `Vec<T>` is edited as
> comma-separated text — a real list widget is a later, non-breaking addition.
> See LOG-0004.

### M3 · Isolation, globals & environment (~2–3 weeks) — 🚧 **IN PROGRESS**
Split manager/preview into two documents + `postMessage` `Channel` impl (no addon changes) · decorators at 3 levels · parameters with merge semantics · globals + toolbar · viewport, backgrounds, measure, outline addons · error boundary reporting panics to the manager.
> **Delivers:** robustness + environment testing. Unblocks everything downstream.
>
> **Done (sessions 0005–0006):** the split, decorators, parameter merging, panic
> reporting, globals + toolbar, and `parameters.layout`. **Not done:** the four
> environment addons, and an `ErrorBoundary` for `Element`-level errors.
>
> A second correction to the scope: globals needed **no new value vocabulary**.
> A `GlobalType` declares the same `Control` a prop's `ArgType` does and the
> values live in an `ArgMap`, so globals inherited a widget, a URL encoding and
> a wire encoding without new code in any of the three. The cost of that reuse
> is one real trap — the URL codec is untyped, so a global from a link needs a
> declared-shape pass (`GlobalType::coerce`) before anything compares it. See
> LOG-0006.
>
> One correction to the scope above, and it is worth reading before touching
> this milestone again. "**Two wasm bundles**" was the wrong shape. The manager
> points its iframe at *its own URL* with `?viewMode=preview` and the same
> bundle loads twice, each copy rendering a different half. That gives the same
> isolation — separate DOM, CSS cascade, JS globals, viewport, reload — with one
> build, no hand-written `iframe.html`, and, decisively, a manager that can still
> see the story registry it needs for the sidebar and the props tables. So
> `ArgType` stays `&'static` on both sides and nothing on the wire needed an
> owned mirror. See LOG-0005.
>
> Two items came forward from later in the plan because the split *created* the
> need for them: a story in a frame inherits none of the app's CSS (decorators
> are the answer), and a story that panics now dies out of sight (a panic hook
> reports it, since `panic = "abort"` rules out catching anything).

### M4 · Docs (~3 weeks)
Autodocs page per component · props table from `ArgType` + doc comments · story source snippet (macro captures the rsx token stream as `&'static str`) · description from component doc comments · docs/canvas tab toggle.
> **Delivers:** the design-system deliverable. Highest external value per unit of work.

### M5 · Static build & publish (~1–2 weeks)
`dx-story build` → static site · index manifest · deep-linkable `iframe.html?id=…&args=…` · CI recipe (GitHub Pages).
> **Delivers:** shareable artifact. Ship this early if adoption matters more than testing.

### M6 · Testing (~4–6 weeks)
`dx-story-testing` DOM queries + user-event · `#[play]` execution + Interactions panel with step log · a11y addon (load axe-core in preview, pipe results over channel, highlight violations) · `dx-story test` headless runner over the static build (render smoke tests + play functions) · visual-regression snapshot + pixel diff.
> **Note:** a11y is disproportionately cheap (axe-core is a JS drop-in) and disproportionately valuable — consider pulling it forward into M3.

### M7 · Extensibility (ongoing)
Public addon trait (manager half + preview half + channel messages) · presets · UI theming · story composition/refs.

### Deliberately deferred
MDX (D4) — needs a Rust MDX pipeline; use plain `.md` + doc blocks instead. Composition/refs (F3), coverage (E7), Figma embeds (F5).

---

## Part 6 — Open decisions

1. ~~**Ambition.**~~ ✅ Settled in M1: **publishable open-source crate**, M0–M6. Consequences now in force: private fields with `const` builders, `#[non_exhaustive]` on enums that will grow, `#![deny(missing_docs)]`, per-crate README + LICENSE.
2. ~~**Registry mechanism.**~~ ✅ Settled in M0: build-script codegen. No longer open.
3. **Sequence bias.** Ship **M5 (publish) early** for adoption and stakeholder buy-in, or **M6 (testing) early** for engineering rigour? Cannot do both first.
4. **Desktop mode.** Is `dx serve --platform desktop` in scope at all, or web-only forever?
5. ~~**Naming / crate namespace.**~~ ✅ Settled in M1: **`dioxus-storybook-*`**, MIT, © DW3Labs. Chosen over `dx-story` for discoverability, accepting the proximity to the Storybook trademark. All five candidate names were free on crates.io at the time.
