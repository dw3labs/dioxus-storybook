# dioxus-storybook-build

The build-script story indexer for
[`dioxus-storybook`](https://crates.io/crates/dioxus-storybook).

```rust,no_run
// build.rs
fn main() {
    dioxus_storybook_build::index("src/stories").unwrap();
}
```

It scans your story files for `#[story]` functions and emits a registry of
explicit `&'static` references into `OUT_DIR`, declaring the story modules as it
goes — so adding a file is enough to add its stories, with no `mod.rs` to keep
in sync.

Codegen rather than a link-section trick, because on `wasm32` `linkme` does not
compile at all and `inventory` silently drops any story in an unreferenced
codegen unit — passing at `codegen-units = 1` and failing at 16 and 256, so
stories vanish in dev builds and reappear in release.

## License

MIT
