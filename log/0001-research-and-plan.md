# 0001 — Research & project plan

**Date:** 2026-09-06 · **Status:** complete · **Outcome:** plan published

## Goal

Understand how Storybook actually works, inventory the features that make it
good, and turn that into a plan for a native Dioxus equivalent.

## What was done

1. Fetched and indexed the Storybook docs (CSF, args, essentials, autodocs,
   testing, main-config, addon types) and current Dioxus docs.
2. Checked prior art on crates.io.
3. Wrote `docs/PLAN.md` and published it as an artifact.

## Findings

### Storybook is three layers, not one tool

- **Authoring format (CSF).** A module with a default export (`meta`) and named
  exports (stories). Six orthogonal concepts are the whole data model: `args`,
  `argTypes`, `parameters`, `decorators`, `globals`, `tags` — each with a
  global → component → story merge order.
- **Build pipeline.** An *indexer* globs story files into a flat index; a
  *docgen* compiler plugin reads prop types and JSDoc to synthesise `argTypes`
  (this is what makes controls "just work"); a *builder* emits two bundles.
- **Runtime.** A manager window and a sandboxed preview iframe talking over a
  `postMessage` channel. Every addon is a pair — manager half renders a panel,
  preview half instruments the render. There is no privileged core: Controls,
  Actions and A11y are all just addons.

The iframe is load-bearing. Because the preview is independently addressable at
`iframe.html?id=…&args=…`, the same URL powers the test runner, screenshot
diffing and embeds.

### What makes it great reduces to four things

Of 36 features inventoried: controls auto-generated from types, stories doubling
as tests, docs generated from the same source, and a channel extensible enough
that everything else is an addon. The other 32 are polish.

### Prior art: none

`storybook` / `storybook-derive` on crates.io (v0.2.2, ~150 downloads combined)
only *bridge* Rust WASM components into the JavaScript Storybook. Nobody has
built a native one.

## Decisions taken

- **Web-first.** The static build is both the shareable artifact and the
  substrate for screenshot testing. Desktop is nearly free later, not a target.
- **Channel abstraction from commit 1.** Run in-process through M2, swap in the
  iframe/postMessage implementation at M3 without rewriting addons.
- **Milestones M0..M7**, ordered so each ships something usable alone.

## Six porting problems identified

| | Problem | Planned answer |
|---|---|---|
| P1 | No runtime module discovery | spike `linkme`, codegen as fallback |
| P2 | No docgen / reflection | own the derive macro |
| P3 | Dynamic args → typed props | generate an applier, not a deserializer |
| P4 | Manager/preview isolation | `Channel` trait first, iframe later |
| P5 | No testing library | small crate over `web-sys` |
| P6 | Multiplatform ambiguity | web-first |

## Artifacts produced

- `docs/PLAN.md` — the working plan
- `docs/plan-artifact.html` → https://claude.ai/code/artifact/2ae04c48-496e-4e46-80ed-0c0272dbf247

## Notes for the next session

The plan asserted the project "rests on Subsecond hot-patching". That claim was
**not verified at this point** — it was flagged as the thing M0 must validate
first. See LOG-0002; it turned out to be wrong in mechanism.
