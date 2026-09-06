# 0007 — M3 pt 3: the viewport addon (M3 complete)

**Date:** 2026-09-06
**Goal:** finish M3 with the last item on the list — the viewport addon.
**Outcome:** shipped. 152 tests + 19 doctests green, `rust-analyzer diagnostics .`
clean, verified by hand in the browser. M3 is done.

---

## Goal

The one remaining M3 item was "viewport / backgrounds / measure / outline". The
HOT section said to do viewport first because "the preview is a frame whose
width the shell owns". That turned out to be the entire design, and the whole
addon is a `<select>`, a style attribute, and one resolution function.

Scope was deliberately held to **viewport only**. Backgrounds is the next
decision (see NEXT SESSION), and measure/outline are preview-side overlays that
share nothing with this.

## What was done

- **`dioxus-storybook-core/src/viewport.rs`** — `Viewport` (name, title, w, h),
  `DEFAULT_VIEWPORTS` (5 sizes), `ViewportSelection` (a viewport + rotation, with
  `width()`/`height()` applying it), `resolve`, `find`,
  `responsive_needs_saying`.
- **`Project::with_viewports` / `viewports()`** — the available list. Defaults to
  `DEFAULT_VIEWPORTS`; `&[]` removes the picker entirely.
- **`StoryContext::viewport()`** — resolved per story, so a decorator can be told
  the size. The `StoryContext` doc had promised this since 0005.
- **`toolbar::ViewportPicker`** — dropdown + rotate button + a size readout,
  rendered next to the globals bar.
- **`PreviewStage`** — the pane the iframe now sits in. Responsive is the plain
  case; a chosen size makes the pane a scrollable backdrop with a fixed-size,
  shadowed frame at the top of it.
- **`examples/button-gallery`** — a `Navbar` component with a real
  `@media (max-width:640px)` in it, and two stories: `Wide`, and `On A Phone`
  which carries `parameters { viewport: "mobile" }`. 15 stories → 17.
- **A keyboard bug fixed** (found in the browser, see F4).

## Findings

### F1 — The addon is a style attribute. Nothing crosses the wire for it.

A viewport *is* the frame's width; the frame belongs to the shell. So choosing
"Mobile" sets `style="width:360px;height:640px"` on the iframe and stops. The
story is not told, not re-rendered, and not handed a breakpoint — its own media
queries fire because it genuinely is that size. Verified in the browser: the
example navbar collapses to a ☰ at 360px and expands at 834px, with nothing in
the preview document aware that anything happened.

This is the dividend from 0005. Before the iframe, "render this story at 360px"
would have meant a wrapper div, a fake breakpoint and a lie.

**Corollary:** `src` must not change. Only the style does. An iframe reloads
when its `src` changes, which would throw away the preview's state and its
channel subscription on every resize — the same rule LOG-0005 wrote for live
state, now with a second reason to obey it.

### F2 — The available list could not be a parameter, and should not have been.

The HOT section predicted "the SELECTED viewport is globals state and the
AVAILABLE LIST is a parameter — copy that split". Half of it survived contact:

- The **selection** is a global. Not a *declared* one, but carried in the same
  `ArgMap`, which is what gives it a URL encoding (`&globals=viewport:tablet`),
  a wire encoding inside `SetGlobals`, a place in `StoryContext::globals()` and
  a reset button — with no new code in any of them. Two names are reserved:
  `viewport` and `viewport-rotated`.
- The **list** could not be a parameter. `ParamValue` is `Str | Num | Bool`, and
  a viewport is a record. Widening `ParamValue` to hold one would have been the
  fourth value vocabulary this project has spent three milestones not growing —
  and the *first* one to be addon-specific. So the list is a declaration on the
  `Project`, next to `globals`, which is where declarations already live.

What *is* a parameter is the half that genuinely varies per story: which size a
story opens at. `parameters { viewport: "mobile" }` on the drawer story, merged
by the same three-level `ResolvedParameters` `layout` uses.

So: **list = declaration, choice = parameter, selection = global.** Three
mechanisms already in the crate, no new ones.

### F3 — "Responsive" has to be a value, not an absence.

Everywhere else in this project, removing a key from the selection map is how
you say "default" — it keeps the link short and lets an empty map mean "nothing
has been changed", which is the question the reset button asks (LOG-0006/G3).

That breaks here. A story with `parameters { viewport: "mobile" }` would be
impossible to look at full width: removing the selection hands the decision
straight back to the parameter. So `responsive` is a real, storable value.

But storing it unconditionally would put `viewport:responsive` in links that
mean nothing by it. Hence `responsive_needs_saying`: the picker stores the word
only when removing the key would resolve to something else. On an ordinary story
picking Responsive leaves the URL clean; on the phone story it writes
`&globals=viewport:responsive`. Both were checked in the browser.

### F4 — The sidebar's keyboard navigation was eating the toolbar's keystrokes.

Found by doing what the working style demands: opening the browser and pressing
a key. Focus the viewport dropdown, press ↓, and the **story** changed
underneath it — the shell listens for arrows/Enter/`/` on the whole `.dxsb`
element, and a `<select>` wants exactly those keys.

Pre-existing (the theme dropdown and the search box had it too since 0006), but
the viewport picker made it the primary control, so it is fixed here:
`onkeydown: swallow_toolbar_keys` on the toolbar header, which stops
propagation. Stopping it at the exception rather than inspecting the event's
target in the outer handler keeps the rule local and needs nothing from the
platform — there is no DOM to ask about a target off `wasm32`.

**No test covers this, and none can here:** a headless `VirtualDom` has no way
to deliver a keystroke. That is a second entry in the "a green suite is not a
working UI" column, alongside LOG-0005's two.

### F5 — `Project::default()` and `Project::new()` had to be made the same thing.

`Project` derived `Default`, and the shell takes it as `#[props(default)]`. Once
`new()` filled in `DEFAULT_VIEWPORTS`, a derived `Default` would have given a
storybook mounted *without* a project an empty picker and one mounted with
`Project::new()` a full one. Written out by hand instead, with a test
(`assert_eq!(Project::default(), Project::new())`) so the two cannot drift.

General shape worth remembering: **the moment a `const fn new()` stops being
all-zeroes, a derived `Default` is a bug waiting for a second constructor.**

### F6 — The example needed a component that changes shape.

Every other component in the gallery looks identical at any width, which makes
the picker look like a picture frame. `Navbar` has one media query and is
therefore the only way to see half the addon. Its stylesheet also carries
`align-self:start`, because the gallery's project decorator centres every story
and a site header centred in a 1112px tablet reads as a bug.

## Decisions

| | Decision | Why |
|---|---|---|
| Where the size lives | **a style attribute on the iframe** | the shell owns the frame; nothing needs to cross the wire (F1) |
| The list | **`Project::with_viewports`** | `ParamValue` cannot hold a record, and widening it would be a fourth vocabulary (F2) |
| The per-story choice | **`parameters { viewport }`** | static, per level, and the half that actually varies per story |
| The selection | **the globals `ArgMap`**, under reserved `viewport` / `viewport-rotated` | URL, wire, `StoryContext` and reset all came free |
| Responsive | **a storable value, written only when it overrides something** | absence already means "whatever the story asked for" (F3) |
| Rotation | **removed rather than stored false** | an unrotated canvas leaves nothing in the link to explain |
| Unknown names | **responsive, never an error** | same rule `canvas_layout` follows: a stale link should land on a canvas |
| `Project::default` | **hand-written as `new()`** | a derived one would silently differ (F5) |

## Files

```
crates/dioxus-storybook-core/src/viewport.rs      NEW  the addon's whole model
crates/dioxus-storybook-core/tests/viewport.rs    NEW  12 tests on `resolve`
crates/dioxus-storybook-core/src/story.rs         Project::with_viewports,
                                                  StoryContext::viewport, manual Default
crates/dioxus-storybook-core/src/lib.rs           module + re-exports
crates/dioxus-storybook-ui/src/toolbar.rs         ViewportPicker
crates/dioxus-storybook-ui/src/lib.rs             PreviewStage, picker wiring,
                                                  swallow_toolbar_keys (F4)
crates/dioxus-storybook-ui/src/style.rs           .dxsb-stage / .dxsb-viewport
crates/dioxus-storybook-ui/tests/two_documents.rs 5 tests across the wire
crates/dioxus-storybook/src/lib.rs                the "Viewports" section
examples/button-gallery/src/nav.rs                NEW  a responsive component
examples/button-gallery/src/stories/nav.rs        NEW  Wide + On A Phone
examples/button-gallery/src/main.rs               mod nav
```

## Verified

- `cargo test --workspace` — 152 tests + 19 doctests, green (was 135 + 16).
- `rust-analyzer diagnostics .` — clean (only `inactive-code` weak warnings from
  `#[cfg(target_arch = "wasm32")]`, as before).
- **In the browser**, `dx serve --platform web`:
  - the picker renders, and `On A Phone` opens at 360×640 with **nothing in the
    URL** — the parameter did it;
  - the navbar is collapsed at 360 and expanded at 834;
  - rotate gives 640×360, the readout follows, the dropdown keeps showing the
    declared 360×640, and the URL carries `globals=viewport-rotated:!true`;
  - a **pasted** `?globals=viewport:tablet` resolves (the untyped-URL path, ie
    `Text` where the picker writes `Variant`);
  - picking Responsive on the phone story writes `viewport:responsive` and goes
    full width; picking it elsewhere writes nothing;
  - the globals reset button clears the viewport with everything else;
  - ↓ in the focused picker no longer changes the story (F4).

## Notes for the next session

M3 is complete. The remaining M3-adjacent items were **deliberately not** done:

- **backgrounds** — decide first whether it is a built-in addon or just the
  documented decorator pattern. The example's theme decorator already does the
  job by hand, and the two would overlap. Do not write both.
- **measure / outline** — preview-side overlays driven by a global toggle. These
  genuinely need the preview, unlike viewport, so they will be the first addon
  to put something *into* the frame rather than around it.
- **`ErrorBoundary`** for a story returning `Err`. Distinct from the panic hook
  (0005), which cannot catch it because wasm here is `panic = "abort"`.
- **The `Callback::new` per-render measurement**, carried since 0004.

One small thing observed and left alone: rotation persists in the URL when the
canvas goes responsive by *changing story*. It is consistent with "globals
outlive the story you set them on", and it comes back correctly when you return.
