# dioxus-storybook-ui

The manager shell for
[`dioxus-storybook`](https://crates.io/crates/dioxus-storybook): sidebar tree,
fuzzy search, keyboard navigation, the preview harness, and URL state.

The manager never calls the preview directly — it emits events on a `Channel`
that the preview subscribes to. Today both halves share one bundle; that
indirection is what lets the preview move into an iframe without any component
here changing.

## License

MIT
