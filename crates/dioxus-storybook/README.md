# dioxus-storybook

A native [Storybook](https://storybook.js.org/) for [Dioxus](https://dioxuslabs.com).

Declare each of your component's states as a *story*, render it in isolation,
and browse the lot in a workbench — with the selected story and its args carried
in the URL, so a rebuild puts you back where you were.

```rust,ignore
story_meta! {
    title: "Forms/Button",
    component: Button,
}

#[story]
fn primary() -> ButtonProps { base() }

#[story]
fn danger() -> ButtonProps {
    ButtonProps { variant: ButtonVariant::Danger, ..base() }
}
```

```console
$ dx serve --platform web
```

See the [crate docs](https://docs.rs/dioxus-storybook) for the full setup, and
`examples/button-gallery` for a working project.

## Status

**M1 — the walking skeleton.** Stories are discovered, indexed, browsable
(sidebar tree, fuzzy search, keyboard navigation) and rendered, and the URL
carries the story id and args. `#[derive(Controls)]` already produces the props
table; the panel that *edits* those controls lands in M2.

## The crates

| Crate | What it holds |
|---|---|
| `dioxus-storybook` | The facade you depend on. |
| `dioxus-storybook-core` | `StoryDef`, args, the index, the channel. No rendering. |
| `dioxus-storybook-macro` | `#[story]`, `story_meta!`, `#[derive(Controls)]`. |
| `dioxus-storybook-build` | The build-script indexer. |
| `dioxus-storybook-ui` | The manager shell and preview harness. |

## License

MIT
