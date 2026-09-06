# 0005 — M3 part 1: isolation, decorators, parameters

**Date:** 2026-09-06 · **Status:** M3 partially complete (3 of 6 items)
**Toolchain:** rustc 1.97.1 · dx 0.7.10 · dioxus 0.7.10 · macOS aarch64

The first and largest slice of M3: put the story canvas in a document of its
own. Two of the six M3 items came along with it, because the split *created*
the need for them — a story in an iframe has none of the app's CSS (decorators),
and a story that panics now dies out of sight (panic reporting).

**Delivered:** 130 tests + 11 doctests, all green. `rust-analyzer diagnostics .`
clean. Verified in a browser, clicking it.

---

## What was built

| | Delivered as |
|---|---|
| The `postMessage` wire format | `core/src/wire.rs` + `core/tests/wire.rs` (9 tests) |
| `ViewMode`, `Event::PreviewReady`, `Event::PreviewPanicked` | `core/src/wire.rs`, `core/src/channel.rs` |
| `ChannelHandle` — a run-time-chosen transport | `core/src/channel.rs` |
| `Subscription` any crate can construct | `core/src/channel.rs` |
| The browser transport | `ui/src/browser.rs::post_message` |
| Role router; `StorybookManager` / `StorybookPreview` | `ui/src/lib.rs` |
| Two-document test harness | `ui/tests/common/mod.rs` |
| The split's own tests | `ui/tests/two_documents.rs` (12 tests) |
| Panic reporting + the notice and its reload | `ui/src/browser.rs`, `ui/src/lib.rs` |
| `Decorator`, `StoryContext`, `Project` | `core/src/story.rs` |
| `ResolvedParameters` — the three-level merge | `core/src/story.rs` |
| `parameters { .. }` / `decorators [ .. ]` on both macros | `macro/src/story.rs` |
| Their tests, builder *and* authored | `dioxus-storybook/tests/decorators.rs` (12) |
| The M3 spike | `spikes/s4-iframe` |

---

## Findings

### F1 — Two *documents*, but one bundle. The plan said two bundles; it was wrong.

`docs/PLAN.md` scoped M3 as "split manager/preview into two wasm bundles +
`postMessage`". Two bundles means two crates, two `dx` builds, a hand-written
`iframe.html`, and — the part that actually matters — a **manager that can no
longer see the story registry**. The HOT section had flagged that as the first
question to answer, because if the manager loses the registry then
`Control::Select`'s `&'static [&'static str]` needs an owned mirror on the wire
and `ArgType` needs a DTO.

The answer is that the question does not have to be asked. The manager points
its iframe at **its own URL** with `?viewMode=preview` appended, and the same
wasm bundle loads a second time and sees the parameter:

```rust
match browser::view_mode() {
    ViewMode::Preview => rsx! { StorybookPreview { .. } },
    _                 => rsx! { StorybookManager { .. } },
}
```

One build, one binary, two documents. Isolation is a property of the *document*
— separate DOM, separate CSS cascade, separate JS globals, independently
resizable viewport, independent reload — and none of that needed a second
compilation unit. `ArgType` stays `&'static` on both sides, nothing on the wire
needs a mirror, and the whole M3 transport question collapses to "write one more
`Channel` impl", which is what the trait was put in at commit 1 for.

**Evidence:** `spikes/s4-iframe`, run under `dx serve --platform web`. It proved
four things before a line of the real thing was written:

1. an iframe with a query-string URL loads the same app — no second HTML file,
   no dx config;
2. parent → child `postMessage` arrives;
3. child → parent arrives;
4. **a plain JS message callback can write a Dioxus signal and the owning
   component re-renders.** That was the one that could have sunk the design.

### F2 — `window.onmessage` is a shared bus, and something else is already on it

The very first browser run of the finished feature had four console messages in
it, all from a wallet extension's content script. Anything a page can be
embedded in — extensions, embeds, dev tooling — posts at that same handler.

So every message carries an envelope, `dxsb1|<sender>|<tag>`, and anything that
does not start with the magic is dropped before it is parsed. This is not
defensive-programming garnish; it is required for correctness on a real page.

### F3 — A document can hear itself, and would answer forever

`window.parent` is `window` when a page is **not** framed. Open the preview URL
directly and its "post to my parent" posts to itself; without a guard the
preview's own `StoryRendered` would come back in as an incoming event.

Hence the sender's `ViewMode` on the wire, and `decode(receiver, raw)` accepting
only `receiver.peer()`. The parameter is the *reader's own* mode, which took one
iteration to get right — the first version took the sender it expected, which
reads backwards at every call site and produced exactly one wrong test.

### F4 — Length-prefix the fields. An action payload is arbitrary `Debug` output.

`Event::ActionLogged`'s payload is whatever the user's type prints. The first
real click in the browser produced:

```
UiEvent { bubble_state: true, ..., data: MouseData { coordinates: Coordinates {
screen: (1089.0, 267.0), client: (821.0, 56.0), ... } } }
```

Braces, colons, commas, parentheses, newlines. A delimiter-split codec mangles
that; `|<byte-len>:<bytes>` cannot. `read_fields` slices with `str::get`, which
returns `None` rather than panicking when a length lands mid-UTF-8 — so a corrupt
length is rejected by the same code that reads a good one.

### F5 — No outbound queue was needed, because every message is a state sync

The obvious worry: the manager mounts and emits `SetCurrentStory` immediately,
and the iframe does not exist yet. The obvious fix is an outbox that flushes on
a handshake.

It is unnecessary. Every manager → preview message (`SetCurrentStory`,
`UpdateArgs`, `ResetArgs`) is *authoritative state*, not a command, and the
preview asks for the current state simply by existing: it emits `PreviewReady`,
the manager bumps a counter, the publishing effect reads that counter and
re-publishes. Dropping the early messages and resyncing is strictly simpler than
queueing them, and it is the same code path that recovers a preview which
reloaded on its own — which `dx serve` makes it do on every source edit.

### F6 — Effects do not run under a bare `VirtualDom`, and that changed the design

`rebuild_in_place()` + `render_immediate_to_vec()` never runs a `use_effect`.
`VirtualDom::process_events()` does, but only once there are no dirty scopes —
so a test pass is *render, then process, then deliver*.

That is a harness detail, but it forced one real design change. The preview's
`PreviewReady` was written as an effect, on the reasoning that hooks run before
children mount and the manager's reply could beat the subscription. It is now a
`use_hook` in `Preview` placed immediately after the `subscribe` hook, which is
correct for the same reason and works in a headless dom as well as a browser.
Ordering *within* a component is enough; nothing needed the renderer.

### F7 — The two-document harness is the real deliverable of this session

`ui/tests/common/mod.rs` stands up two independent `VirtualDom`s joined by a
channel pair that carries **only encoded strings** — the real codec is in the
loop. It reproduces two properties of `postMessage` on purpose:

- **a document does not hear itself** (unlike `InProcessChannel`, which
  broadcasts to the emitter too), so any half that starts relying on its own
  echo fails here and not in a browser;
- **delivery is asynchronous**: an emit enqueues, and delivery happens between
  render passes. Doing it synchronously would also write one dom's signals from
  inside the other dom's runtime, which is not a thing a browser ever does. Each
  delivery runs inside the *recipient's* runtime, via `dom.in_runtime`.

The old tests were then pointed at the right document — and one of them turned
out to have been passing for a silly reason: `story_bodies_are_invoked_in_scope`
asserted the shell contained `"ok"`, and `"dioxus-storybook"` contains `"ok"`.
Once the story moved out of the manager's document the assertion should have
failed and did not.

### F8 — The split silently broke two things. Both are now features.

Isolating the preview takes away two things nobody asked to lose:

1. **The story's CSS and assets.** The frame inherits nothing. The answer is
   *decorators*, which is why they came in this session rather than later: a
   project-level decorator renders **inside the preview document**, so it is
   where a stylesheet link, a font, or a theme provider belongs.
2. **Visibility of a panic.** Before M3 a panicking story took the page down,
   loudly. Now it kills the frame and leaves a healthy-looking workbench beside
   a rectangle that has quietly stopped. So the preview installs a panic hook
   that posts `Event::PreviewPanicked` on the way out, and the shell shows the
   message with a "Reload the preview" button.

There is no automatic recovery to offer for (2): wasm here is `panic = "abort"`
(a hard `dx` requirement — STANDING FACT from M0), so the module is *gone*, not
unwound. `catch_unwind` is not an option; saying so before dying is the whole of
what can be done.

### F9 — `set_hook` demands `Send + Sync`; the channel is `Rc`

`std::panic::set_hook` takes `Box<dyn Fn(&PanicHookInfo) + Sync + Send>`, and
`ChannelHandle` is `Rc`-based and deliberately not thread-safe. The hook
therefore captures **nothing** but the previous hook, and looks `window` up
afresh inside the body — which satisfies the bound without making the transport
thread-safe. It chains rather than replaces, because Dioxus installs a hook that
prints to the console and losing that would trade one silence for another.

### F10 — Percentage heights do not resolve through the host app's mount element

The framed canvas rendered top-aligned rather than centred. `min-height: 100%`
resolves against the parent chain, and the chain is
`html → body → div#main (the dx mount point, auto height) → .dxsb-canvas`.
`100vh` is the only height a preview stylesheet can rely on, because it does not
know what its host mounted into.

Measured, not guessed: `getComputedStyle` in the frame reported
`{parentTag: "DIV#main", parentH: 115, minH: "100%"}`.

### F11 — Merging parameters needs no allocation

`ResolvedParameters` keeps the three `Parameters` side by side and consults them
story → component → project on `get`. It stays `Copy` and `const`-constructible,
which matters because parameters are `&'static` tables and the whole point was to
avoid a runtime map. `iter()` deduplicates so each key appears once, with its
winning value.

Reading a parameter as the wrong type yields the default rather than panicking:
a typo three levels up must not be able to take an addon down.

---

## Decisions

| # | Decision | Why |
|---|---|---|
| — | **One bundle, two documents** (not two bundles) | F1. Isolation is a property of the document; a second build unit buys nothing and costs the shared registry |
| — | Envelope + sender on every message | F2, F3 |
| — | Length-prefixed fields, hand-rolled, no `serde` | F4. Nine tags over a closed vocabulary; `ArgMap` already had a tested textual codec (the URL's) |
| — | Drop pre-handshake messages, resync on `PreviewReady` | F5. Every manager → preview message is state, not a command |
| — | The manager **always** frames the preview, on every target | The shipped arrangement is the tested one; no `cfg` divergence between what runs and what the suite sees |
| — | `StorybookManager` / `StorybookPreview` are public | The split is only testable if the halves are separately mountable, and M7's addon API wants this shape anyway |
| — | Decorators are `fn(&StoryContext, Element) -> Element` | A context struct now, rather than widening the signature later, which would be breaking |
| — | `StoryDef` carries its whole `Meta`, not copied fields | Every later addition to `Meta` reaches stories without `StoryDef` or the macro growing a line |
| — | No `sandbox` attribute on the iframe | Permissive enough to run the story and talk to it is permissive enough to be decoration. The stories are the developer's own code, in the same binary |

---

## Files produced

```
spikes/s4-iframe/                          the M3 feasibility spike
crates/dioxus-storybook-core/src/wire.rs   the codec, ViewMode, the envelope
crates/dioxus-storybook-core/tests/wire.rs 9 tests
crates/dioxus-storybook-ui/tests/common/mod.rs   the two-document harness
crates/dioxus-storybook-ui/tests/two_documents.rs 12 tests
crates/dioxus-storybook/tests/decorators.rs      12 tests
examples/button-gallery/src/stories/diagnostics.rs  a story that panics on request
```

Changed: `core/src/channel.rs` (`ChannelHandle`, `Subscription::new`, two new
events), `core/src/story.rs` (decorators, `Project`, `ResolvedParameters`),
`core/src/lib.rs`, `ui/src/browser.rs` (transport + panic hook),
`ui/src/lib.rs` (role router, the two halves, the death notice),
`ui/src/style.rs` (`PREVIEW_CSS`), `macro/src/story.rs` (both new keys),
`dioxus-storybook/src/lib.rs` (docs + re-exports), `examples/button-gallery`.

---

## Notes for the next session

Three M3 items remain: **globals + toolbar**, the **viewport / backgrounds /
measure / outline addons**, and the **error boundary** for `Element`-level
errors (distinct from the panic hook, which is done — `ErrorBoundary` catches a
story returning `Err`, which does not abort).

The addons are what `ResolvedParameters` was built for and nothing reads it yet;
`viewport` is the natural first consumer, and it is nearly free now that the
preview is a frame whose width the shell owns.

`Event` is `#[non_exhaustive]` and `wire.rs` matches it **without a wildcard**,
so a new variant fails to compile until it is given a wire form. That is
deliberate — it is the only thing stopping a new message from silently going
nowhere. Do not add a `_ =>` arm to `encode`.

Still unmeasured, carried over from LOG-0004: `Callback::new` runs once per
action prop per render inside `wire_actions`. Entries free only when `StoryHost`
remounts. Bounded, but drag a slider and find out.
