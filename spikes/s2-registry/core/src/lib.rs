//! S2 spike: can a `linkme` distributed slice survive a wasm32 release build?
//!
//! This mirrors the shape of the real `dx-story-core` registry so the answer
//! transfers directly. The risk is that `--gc-sections` (lld does this by
//! default on wasm) or `wasm-opt` drops the link section, because nothing in
//! the call graph ever *references* the registered statics by name.

use linkme::distributed_slice;

/// Mirrors the eventual `dx_story_core::StoryDef`.
#[derive(Debug)]
pub struct StoryDef {
    pub id: &'static str,
    pub title: &'static str,
    pub name: &'static str,
    /// Stand-in for the real `fn(&ArgMap) -> Element` render pointer.
    /// Including a fn pointer matters: it is what forces the story body to be
    /// kept alive alongside the metadata.
    pub render: fn() -> u32,
}

#[distributed_slice]
pub static STORIES: [StoryDef];

/// Re-export so downstream crates need not depend on linkme directly.
pub use linkme::distributed_slice as register;

pub fn count() -> u32 {
    STORIES.len() as u32
}

/// Sum of every registered story's render fn. Proves the entries are not just
/// present but *callable* — i.e. the bodies survived too, not only the table.
pub fn checksum() -> u32 {
    STORIES.iter().map(|s| (s.render)()).sum()
}

/// Registered in the same crate that declares the slice.
#[distributed_slice(STORIES)]
static CORE_LOCAL: StoryDef = StoryDef {
    id: "core--local",
    title: "Core",
    name: "Local",
    render: || 1,
};
