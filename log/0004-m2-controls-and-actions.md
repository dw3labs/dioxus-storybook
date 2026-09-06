# 0004 — M2: controls & actions

**Date:** 2026-09-06
**Goal:** the differentiating milestone. Make the workbench editable: a controls
panel generated from the props type, an actions panel fed by the component's own
event handlers, and a way for a story to drive its own controlled inputs.
**Outcome:** M2 complete. 67 tests + 10 doctests, all green.
`rust-analyzer diagnostics .` clean. wasm builds.

---

## What was built

| | Delivered as |
|---|---|
| Controls panel, all seven widgets | `dioxus-storybook-ui/src/panels.rs` |
| Reset to story defaults, per-row unset | same, emitting `Event::ResetArgs` |
| Actions panel | same, fed by `Event::ActionLogged` |
| `EventHandler` auto-wiring | `Controllable::wire_actions`, emitted by `#[derive(Controls)]` |
| `use_args` write-back | `dioxus-storybook-core/src/actions.rs` + `Event::RequestArgsUpdate` |
| `ArgValue::List` / `Vec<T>` | `core/src/args.rs`, `core/src/url.rs` |
| A story with its own scope | `StoryHost` in `dioxus-storybook-ui/src/lib.rs` |

The example grew a third component (`Field`) chosen specifically because a
button cannot demonstrate either of the hard halves: a handler carrying a real
payload, and a *controlled* input that only works if the story writes an arg
back. 14 stories now.

---

## Findings

### F1 — The manager must not compute a story's default args. It cannot.

The controls panel needs two things per prop: the shape (which widget) and the
current value. It is tempting to read both off `StoryDef` in the manager.

Shape is fine — `arg_types()` is a `const` table with no runtime requirements.
**Value is not.** `base_args()` evaluates the story's props, which builds
`EventHandler`s, which needs a live Dioxus scope (STANDING FACT from M0). Worse,
at M3 the story functions are in the *preview's* bundle, so a manager that calls
them has nothing to call.

So the preview computes them once per story mount and publishes them:

```rust
Event::StoryPrepared { id, initial_args }
```

`ArgMap` is owned data, so this message survives the M3 transport change
unaltered. Deliberately *not* done: putting `ArgType` on the wire. It holds
`&'static str` and `&'static [&'static str]`, which cannot cross a `postMessage`
boundary — and does not need to, because the manager can read the const table
directly in either architecture.

The `control_values_arrive_from_the_preview_over_the_channel` test asserts this
whole round trip: a default appearing in the rendered panel can only have come
across the channel.

### F2 — Stories needed a scope of their own, and `key` alone does not give one

M1 rendered story bodies inline in `Preview`:

```rust
section { class: "dxsb-canvas", {def.render(&overrides)} }
```

That puts any hook a story calls into **the preview's** hook list. Two stories
that call a different number of hooks then corrupt each other on a switch —
Dioxus hands the second story a hook slot belonging to the first and panics on
the downcast. `use_args` made this urgent (a controlled story wants local state),
but the trap was already there in M1 for any story that called `use_signal`.

The fix is a `StoryHost` component remounted per story. **The remount is the
subtle part.** Dioxus only consults `key` inside `diff_keyed_children`, i.e. when
diffing a *list*. A lone keyed child sitting in a fixed template position is
diffed by `diff_node`, which never looks at the key — it would be re-rendered in
place, hooks intact, bug unfixed.

Hence the deliberately odd-looking:

```rust
for def in [def] {
    StoryHost { key: "{def.id()}", story: def, args: overrides.clone() }
}
```

A keyed list of exactly one. `switching_stories_remounts_the_host_instead_of_reusing_its_hooks`
proves it by mounting two stories with different hook counts and asserting both
that the second renders and that the host mounted twice.

### F3 — Action payloads print without putting `Debug` on anyone's types

Wiring `EventHandler<T>` to the actions panel means rendering `T`. Requiring
`T: Debug` would be a bound imposed on other people's prop types purely because
they wanted a panel — unacceptable for a published crate.

Solved with autoref specialisation (dtolnay's technique), in
`core::actions::describe`: an **inherent** method on `Probe<&T>` bounded by
`T: Debug`, and a **trait** method as the fallback. Inherent methods are tried
first, so the printable case wins when the bound holds and falls through to `"…"`
when it does not. Verified in a standalone spike before adoption, then again
through generated code in
`a_payload_without_debug_degrades_instead_of_failing_to_compile`.

### F4 — Wrapping handlers works for `Callback<A, R>`, not just `EventHandler<A>`

The generated wrapper is built with `<#field_ty>::new(..)`, not
`EventHandler::new(..)`. `Callback::new` takes `impl FnMut(Args) -> MaybeAsync`
where a blanket `SpawnIfAsync` impl covers the plain case, so a closure that
returns whatever the inner handler returns type-checks for any `R`. Naming the
field's own type is what makes that inference work.

`Option<EventHandler<T>>` is handled too, via `.map()`. Covered by
`wire_actions_reports_calls_and_still_calls_the_storys_own_handler`, which also
asserts the story's own handler still runs — instrumentation observes, it does
not replace.

### F5 — Two arg messages, travelling in opposite directions

Storybook has both `updateStoryArgs` (a partial *request*) and
`storyArgsUpdated` (a full *broadcast*). M1 had only the broadcast. `use_args`
needs the request, because a story knows the one arg it just changed and nothing
about the rest of the set — and a handle built at mount time would capture a
stale full map anyway.

- `Event::UpdateArgs { id, args }` — manager → preview, authoritative and complete.
- `Event::RequestArgsUpdate { id, args }` — preview → manager, a delta to merge.

The manager merges and re-broadcasts, so there is exactly one place that decides
what the current arg set is. The merge is guarded (`if current != merged`) — the
re-broadcast lands back in the same listener, and an unconditional `set` would
spin the effect forever.

### F6 — `try_consume_context` panics without a runtime

Not just without a *context* — `Runtime::with_current_scope` unwraps
`Runtime::current()`, which panics. `ActionSink::ambient()` is called from
generated code in **every** story body, so "there is no runtime" has to be an
answer rather than a crash. Both ambient lookups now go through a helper that
checks `Runtime::try_current()` first. Found by a test that asserted the
forgiving behaviour and got a panic instead.

### F7 — `title=""` is not "no tooltip"

Reported from the running app mid-session: the example button's tooltip did
nothing. The args pipeline turned out to be correct end to end (verified by
rendering all four cases). The bug was in the example component:
`props.tooltip.clone().unwrap_or_default()` emits `title=""` for `None`, and a
native tooltip additionally (a) waits 1–3 seconds, which reads as "the control
did nothing" when you are editing the prop live, and (b) is suppressed entirely
on a `disabled` control, so the `Disabled` story could never have shown one.

Passing the `Option` straight in (`title: props.tooltip.clone()`) omits the
attribute — but the delay and the disabled case are the platform, not us. The
example now draws a CSS bubble on a wrapper span instead, which is instant and
survives `disabled`. The CSS lives with the component, not in `MANAGER_CSS`:
from M3 the preview is a separate document the manager's stylesheet cannot
reach into.

### F8 — Two bugs that only a running browser showed

The panels were green under SSR and a real `VirtualDom`, and still had two
defects that only appeared once driven by hand:

**A signal-ownership warning, 74 times in one minute.** `Preview` created its
`current` and `args` signals with `use_signal`, so they were owned by *its*
scope — but the writer is the channel listener, which the manager invokes from
`Storybook`'s scope, an ancestor. dioxus-signals warns about exactly that
("a Copy value ... used in a scope which is not a descendant of the owning
scope") because the value can be dropped while an outer scope still holds it.

Benign today — `Preview` never unmounts before `Storybook` — but it fired on
every keystroke and would bury a real warning. Fixed with
`Signal::new_in_scope(.., ScopeId::ROOT)`: ownership moves to the root, and the
preview still learns everything over the channel rather than having its state
hoisted into the manager. **This was M1 code**; M2 only made it loud.

**A story that wrote args back but had nowhere to show them.** `Controlled` was
written in the `Element` form, which opts out of the props table — so `use_args`
worked (the URL moved) but the controls panel showed the "no props table" empty
state, and the feature demonstrated itself invisibly. A story body runs inside
the preview's scope, so the **props form can call `use_args` just as well**.
Rewritten that way, a keystroke now moves the field, the `value` control row and
the address bar together, and the wrapped handler logs to Actions at the same
time.

Lesson for the milestone: SSR proves the markup, not the runtime. Neither of
these was reachable without clicking.

### F9 — Two DevTools reports: one ours, one not

Both came from the running app's Issues panel.

**"A form field element should have an id or name attribute" — ours.** Every
control widget now carries `id="dxsb-ctrl-{prop}"` and `name="{prop}"`, and the
prop name in the first column became a real `<label for=..>`. Worth more than
silencing a warning: the accessibility tree went from unnamed controls to
`textbox "label"`, `combobox "Neutral"`, `checkbox "disabled"`.

**"Incorrect use of `<label for=FORM_ELEMENT>`", 2 resources — also ours, and
caused by the fix above.** Emitting `for` unconditionally pointed at nothing on
the rows that render prose rather than a widget: an `EventHandler` prop
(`Control::Action`) or a skipped one (`Control::None`). On the `Field` story that
is `oninput` and `onblur` — exactly the two reported. A radio row was wrong too,
for a different reason: its inputs are keyed `{field_id}-{option}`, so plain
`field_id` matched nothing there either.

Now `label_target` decides per control: `None` for Action/None (render a `span`),
the first option's id for a radio group, `field_id` for everything else. Verified
in the page — `label[for]` with no matching element: 0, and form controls missing
both id and name: 0.

**"Content Security Policy blocks the use of 'eval'" — not ours.** Measured
rather than assumed: the app serves no CSP header and no CSP `<meta>` (only
`Content-Type`), and `eval('1+1')` succeeds on the page. The report belongs to
another frame in that tab — an extension; MetaMask's content script was also
logging there.

There *is* one `new Function(...)` in the wasm-bindgen glue, and it is worth
knowing where it comes from: `dioxus-web-0.7.10/src/document.rs:220`, which is
`WebEvaluator::create`, the implementation of `document::eval`. It is linked into
every dioxus-web binary but only *runs* when something calls `document::eval` —
which nothing in these crates or the example does. So a storybook deployed behind
a strict CSP without `unsafe-eval` is fine today. **Re-check at M5** (static build
and deploy), and again if anything ever reaches for `document::eval`.

---

## Verified in the browser

Driven by hand at `localhost:8080`, all confirmed:

| | |
|---|---|
| Text/select/range/toggle/colour widgets | render with the story's own values |
| Editing a control | re-renders the preview, marks the row overridden, writes the URL |
| `disabled` toggle | encodes as `args=disabled:!true` |
| Reset to story defaults | clears args, strips the query, disables itself again |
| Actions, `EventHandler<String>` | logs `"h"`, `"he"`, `"hey"` — `Debug` payloads |
| Actions, `EventHandler<MouseEvent>` | logs the full `MouseData` with coordinates |
| `use_args` write-back | field, `value` row and URL move together, no feedback loop |
| Actions log | clears on story switch |
| Tooltip | instant, and still shown on a `disabled` button |
| Console | clean (only MetaMask's content script) |
| Form controls | every one has an id and a name; no `label[for]` dangles |

---

## Decisions

- **`wire_actions` is a method on `Controllable`, not a second trait.** The trait
  is derive-only by documentation, and splitting it would only add ceremony to a
  crate that has not shipped a version yet.
- **The controls panel edits the manager's own signal**, not the preview's, and
  not a shared one. It never touches the preview — the emit in the existing
  effect is what carries the change. This is the constraint HOT set in M1 and it
  survived contact.
- **`Vec<T>` is edited as comma-separated text** rather than getting a widget of
  its own. `FromArg for Vec<T>` accepts both a real `ArgValue::List` and the text
  form, so the round trip is honest. `Control` is `#[non_exhaustive]`, so a real
  list widget is a non-breaking addition later.
- **The URL list form is `!,a,b,c`**, joining the existing `!true`/`!false`/
  `!null` literal convention. Lossy in exactly one place: an empty list and a
  list holding one empty string both encode as `!,`. Documented; the cheaper end
  of the trade against a heavier syntax.

---

## Known issue for M3

`Callback::new` is documented as "should not be called directly in the body of a
component because it will not be dropped until the component is dropped".
`wire_actions` builds one handler per action prop **per render**, and a story's
own `base()` already built one per render in M1. Dragging a slider therefore
accumulates generational-box entries until the story is switched (which now
remounts `StoryHost` and frees them). Bounded, but not free. Worth measuring
before M3 rather than assuming it is fine.

---

## Files

**New**
```
crates/dioxus-storybook-core/src/actions.rs        ActionSink, ArgsHandle, use_args, describe
crates/dioxus-storybook-core/tests/actions.rs      7 tests
crates/dioxus-storybook-ui/src/panels.rs           controls + actions panels
crates/dioxus-storybook-ui/tests/panels.rs         8 tests
crates/dioxus-storybook/tests/controls_and_actions.rs  9 tests
examples/button-gallery/src/field.rs               component that needs M2 to be demoable
examples/button-gallery/src/stories/field.rs       5 stories, incl. the use_args one
log/0004-m2-controls-and-actions.md                this file
```

**Changed**
```
core/src/args.rs        ArgValue::List, as_text/as_num/as_bool, Vec<T>, wire_actions
core/src/arg_type.rs    (unchanged — the widget vocabulary was already right)
core/src/channel.rs     StoryPrepared, RequestArgsUpdate
core/src/url.rs         list codec
macro/src/props.rs      Vec inference, wire_actions emission
macro/src/story.rs      render body calls wire_actions with the ambient sink
ui/src/lib.rs           StoryHost, panel wiring, listeners
ui/src/style.rs         panel CSS
dioxus-storybook/src/lib.rs   prelude exports, status docs
examples/button-gallery/src/button.rs  real tooltip
```

---

## Notes for the next session

M3 is next: split the manager and preview into two wasm bundles with a
`postMessage` `Channel`. The M2 work was written against that split — no addon
reads a preview signal, the manager never evaluates a story, and every message on
the bus carries owned data. The one thing to check first is whether `ArgType`
really can stay `&'static` in both bundles, which depends on whether the manager
bundle still links the registry.

Everything above was driven by hand in Chrome at the end of the session — which
is how F8 was found. Keep doing that: this milestone had two defects that were
invisible to 92 green tests.
