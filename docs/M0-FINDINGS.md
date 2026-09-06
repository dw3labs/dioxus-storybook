# M0 — Feasibility spike results

> Run 2026-09-06 · rustc 1.97.1 · dx 0.7.10 · dioxus 0.7.10 · macOS aarch64
> Reproduce: `spikes/s2-registry/verify.sh`, `cargo test -p s3-test`, `/tmp/s1_test2.sh`

## Verdict: **GO** — with one assumption corrected and one plan change.

| Spike | Question | Result |
|---|---|---|
| S1 | Does the dev loop stay under 2s? | **PASS**, but not for the reason the plan assumed |
| S2 | Can stories self-register on wasm? | **NO** — codegen is the only reliable option |
| S3 | Can a derive macro replace docgen? | **PASS**, cleanly |

---

## S1 — Hot reload

### Subsecond Rust hot-patching is currently broken on wasm

`dx serve --hot-patch` fails to build:

```
ERROR Build failed: Failed to emit module: failed to parse import section
  2: exceptions proposal not enabled (at offset 0x33d24)
```

Isolated by A/B: `dx serve` without the flag builds fine; with it, always fails.
The `--hot-patch` path produces a *fat archive* (`libdeps-*.a`) keeping every
dependency object that normal linking would garbage-collect — including std
objects carrying wasm exception-handling opcodes — and dx's own module emitter
parses without the exceptions feature enabled.

`panic = "abort"` in `[profile.wasm-dev]` does **not** fix it (identical failure,
identical offset). `exception-handling` is not a default rustc target feature for
wasm32, so this is a dx-versus-current-std incompatibility, not a project config
error. Worth filing upstream; not worth blocking on.

Also noted: `--hot-patch` defaults to **false**, and `dx` refuses to run when its
version differs from the `dioxus` crate version (0.7.9 vs 0.7.10 errored).

### But the loop is fast anyway

Measured against a running `dx serve` (edits applied in place — note that macOS
`sed -i ''` writes a sibling temp file that dx's watcher latches onto instead,
which will corrupt any naive measurement):

| Edit | Path taken | Time |
|---|---|---|
| rsx text node (markup) | rsx hot reload, no rebuild | **0.36s** |
| Rust logic outside rsx (match arm) | full incremental rebuild | **1.52s** |
| Story data (`stories.rs`) | full incremental rebuild | **1.49s** |
| Cold start | full build | 19–32s |

**The plan's load-bearing assumption was wrong in its mechanism but right in its
conclusion.** The project does not depend on Subsecond. It depends on *the
incremental rebuild being ~1.5s*, which it is. Subsecond becomes an optimisation
to adopt when it works on wasm, not a precondition.

### Consequence: URL state moves to the foundation

A rebuild reloads the page, so every in-memory signal is lost — selected story,
edited args, scroll position. Feature **A4 (deep-linkable URLs encoding story id
+ args) is therefore not a sharing nicety; it is the mechanism that makes the dev
loop survive a rebuild.** It must land in M1, not M2.

---

## S2 — Story registration

Same 3-crate fixture (4 stories, checksum 131) through every mechanism, with the
resulting wasm actually executed rather than merely built:

| Mechanism | Release | Debug (`dx serve`) | Verdict |
|---|---|---|---|
| `linkme` | does not compile | does not compile | **Unusable** |
| `inventory` | PASS | **FAIL (2/4)** | **Unreliable** |
| build-script codegen | PASS | PASS | **Adopted** |

- **`linkme` 0.3.37** rejects wasm32 outright: `distributed_slice is not
  implemented for this platform`. The identical code compiles for the host, so
  this is a platform gap, not a usage error.
- **`inventory` 0.3.24** compiles and works — until it doesn't. Registration is
  lost for any story in a codegen unit nothing else references. Bisected on
  `codegen-units`: **1 → PASS, 16 → FAIL, 256 → FAIL**. Release (LTO, CGU=1)
  passes; dev (CGU=256) drops stories. It fails in the worst possible direction:
  **stories vanish silently while you develop and reappear in production.**
- **Codegen** passes under default hostile settings in both profiles.

### Methodology note worth keeping

The first `inventory` run passed with a 916-byte module — LTO had const-folded
the whole registry into a literal. Any future registry test must defeat this:
wrap accessors in `core::hint::black_box` and reconstruct story ids byte-by-byte
out of linear memory, so a pass proves the data genuinely exists at runtime.

---

## S3 — Props introspection

**15/15 tests pass** against real Dioxus types — the `Props` derive, `Element`,
and `EventHandler<MouseEvent>` — not stand-ins.

`#[derive(Controls)]` correctly produced:

- doc comments as prop descriptions;
- enum → `Select` with real variant names (via `#[derive(ControlEnum)]`);
- `Option<T>` → optional, `Null` clears it;
- `Element` → no control; `EventHandler` → `Action`, excluded from seeded args;
- `#[control(range(...))]` and `#[control(color)]` overriding inference;
- an applier that overlays only supplied args and leaves the rest typed.

Garbage input (`scale: "not-a-number"`, `variant: "Nonexistent"`) falls back to
the story's typed default rather than panicking — important, since args arrive
from a URL.

**P2 is confirmed as an advantage, not a workaround:** the macro sees names,
types and docs statically, and the applier it emits is compiler-checked.

### Design constraint discovered

`rsx!` and `EventHandler::new` require an **active Dioxus scope**, not merely a
runtime (`VirtualDom::in_scope`, not `in_runtime`). Story bodies therefore cannot
be eagerly-evaluated `fn() -> Props`; `StoryDef` stores fn pointers invoked from
inside the preview component. The S1 app is already built this way.

---

## Changes to the plan

1. **P1 settled:** build-script/CLI codegen is the registry, with evidence. Drop
   the `linkme` option entirely.
2. **Restate the Subsecond dependency:** the project rests on ~1.5s incremental
   rebuilds, not on hot-patching. Remove it as a precondition.
3. **Move A4 (URL-encoded story id + args) from M2 into M1** — it is what makes
   state survive the rebuild-and-reload loop.
4. **Add to M1:** `StoryDef` holds fn pointers, invoked in-scope.
5. **Pin toolchain:** `dx` and `dioxus` versions must match exactly; document it.

Nothing here changes the milestone ordering or the overall shape of the project.
