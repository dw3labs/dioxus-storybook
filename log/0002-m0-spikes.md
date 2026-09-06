# 0002 — M0 feasibility spikes

**Date:** 2026-09-06 · **Status:** complete · **Verdict: GO**
**Toolchain:** rustc 1.97.1 · dx 0.7.10 · dioxus 0.7.10 · macOS aarch64

Three go/no-go questions, answered by running code rather than reading docs.

| Spike | Question | Result |
|---|---|---|
| S1 | Does the dev loop stay under 2s? | PASS — but not via Subsecond |
| S2 | Can stories self-register on wasm? | NO — codegen is the only option |
| S3 | Can a derive macro replace docgen? | PASS, cleanly (15/15) |

---

## S1 — Hot reload

**Location:** `spikes/s1-hotreload/` (excluded from the workspace — `cd` in)
**Run:** `cd spikes/s1-hotreload && dx serve --platform web`

A real vertical slice: sidebar, live preview, an Actions panel, and a Controls
panel whose every widget is generated from the S3 macro rather than hand-written.

### Subsecond hot-patching is BROKEN on wasm

```
$ dx serve --platform web --hot-patch
ERROR Build failed: Failed to emit module: failed to parse import section
  2: exceptions proposal not enabled (at offset 0x33d24)
```

- Isolated by A/B: without the flag the same project builds fine.
- The `--hot-patch` path writes a *fat archive* (`libdeps-*.a`) keeping every
  dependency object that normal linking would GC — including std objects
  carrying wasm exception-handling opcodes — and dx's own module emitter parses
  without the exceptions feature enabled.
- **`panic = "abort"` in `[profile.wasm-dev]` does NOT fix it** — identical
  failure, identical offset. Tried and ruled out; do not retry.
- `exception-handling` is not a default rustc target feature for wasm32
  (`rustc --print cfg --target wasm32-unknown-unknown` confirms), so this is a
  dx-versus-current-std incompatibility, not a project misconfiguration.
- Also: `--hot-patch` defaults to **false**, and it is a `serve`-only flag
  (`dx build --hot-patch` is "unexpected argument").

### But the loop is fast anyway

| Edit | Path taken | Time |
|---|---|---|
| rsx text node (markup) | rsx hot reload, no rebuild | **0.36s** |
| Rust logic outside rsx (match arm) | full incremental rebuild | **1.52s** |
| Story data in `stories.rs` | full incremental rebuild | **1.49s** |
| Cold start | full build | 19–32s |

### THE CORRECTION THAT MATTERS

The plan said the project rests on Subsecond preserving state across a hot patch.
**It does not.** It rests on the incremental rebuild being ~1.5s — which it is,
today, with hot-patching switched off entirely. Subsecond is an optimisation to
adopt once it works on wasm, not a precondition.

### Consequence: URL state moves into M1

A rebuild reloads the page, and a reload destroys every in-memory signal —
selected story, edited args, scroll. So feature **A4 (deep-linkable URLs
encoding story id + args) is not a sharing nicety; it is the mechanism that lets
the dev loop survive a rebuild.** Moved from M2 into M1.

---

## S2 — Story registration

**Location:** `spikes/s2-registry/` · **Run:** `./spikes/s2-registry/verify.sh`

Same fixture throughout: 4 stories across 3 crates, checksum 131. The resulting
wasm is **executed** (node runner, `spikes/s2-registry/runner/run.mjs`), not
merely compiled.

| Mechanism | Release | Debug (`dx serve`) | Verdict |
|---|---|---|---|
| `linkme` 0.3.37 | won't compile | won't compile | **Unusable** |
| `inventory` 0.3.24 | PASS | **FAIL (2/4)** | **Unreliable** |
| build-script codegen | PASS | PASS | **ADOPTED** |

- **`linkme`** rejects the target outright: `distributed_slice is not
  implemented for this platform`. The identical code compiles for the host, so
  it is a platform gap, not a usage error.
- **`inventory`** is the dangerous one. Registration is lost for any story in a
  codegen unit nothing else references. Bisected on `codegen-units`:
  **1 → PASS, 16 → FAIL, 256 → FAIL.** Release uses 1 unit and passes; dev uses
  256 and drops stories. It fails in the worst direction — *stories vanish
  silently while you develop and reappear in production.*
- **Codegen** passes in both profiles under default hostile settings. The
  build.rs scans `src/stories/**.rs` for `story!(IDENT, ...)` and emits explicit
  `&'static` references, which no linker decision can drop.

### METHODOLOGY TRAP — read before writing any registry test

The first `inventory` run **passed**: a 916-byte module reporting exactly the
right count. LTO had const-folded the whole registry into a literal; the test
proved nothing. The fix, now baked into the spike: wrap accessors in
`core::hint::black_box`, and reconstruct each story id **byte-by-byte out of
linear memory** (`story_id_byte(i, j)`), so a pass requires the data to
genuinely exist at runtime.

---

## S3 — Props introspection

**Location:** `spikes/s3-controls/` · **Run:** `cargo test -p s3-test` → 15/15

Tested against **real** Dioxus types — the `Props` derive, `Element`,
`EventHandler<MouseEvent>` — not stand-ins.

`#[derive(Controls)]` correctly produced:

- doc comments as prop descriptions;
- enum → `Select` with real variant names (via `#[derive(ControlEnum)]`);
- `Option<T>` → optional; `ArgValue::Null` clears it;
- `Element` → no control; `EventHandler` → `Action`, excluded from seeded args;
- `#[control(range(min,max,step))]` and `#[control(color)]` beating inference;
- an applier overlaying only supplied args and leaving the rest typed.

Garbage input (`scale: "not-a-number"`, `variant: "Nonexistent"`) falls back to
the story's typed default rather than panicking — important, since args arrive
from a URL.

**P2 is confirmed as an advantage, not a workaround:** the macro sees names,
types and docs statically, and the applier it emits is compiler-checked.

### Design constraint discovered

`rsx!` and `EventHandler::new` require an active Dioxus **scope**, not merely a
runtime — `VirtualDom::in_scope(ScopeId::ROOT, ..)`, not `in_runtime`. So story
bodies cannot be eagerly-evaluated `fn() -> Props`; `StoryDef` stores fn
pointers invoked from inside the preview component. The S1 app already does this.

---

## Changes made to the plan

1. **P1 settled** — build-script codegen is the registry. Drop `linkme` entirely.
2. **Subsecond dependency restated** — the project rests on ~1.5s incremental
   rebuilds, not on hot-patching.
3. **A4 moved M2 → M1** — URL state is what survives the rebuild loop.
4. **M1 addition** — `StoryDef` holds fn pointers, invoked in-scope.
5. **Toolchain pinned** — `dx` and `dioxus` versions must match exactly.

Milestone ordering and overall shape are unchanged.

## Files produced

```
docs/M0-FINDINGS.md              full write-up
spikes/s2-registry/verify.sh     reproduces the registry matrix
spikes/s2-registry/runner/       node wasm runner + section dumper
spikes/s3-controls/              core + proc macro + 15 tests
spikes/s1-hotreload/             the vertical slice app
```
