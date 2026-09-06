//! A separate crate registering stories. This is the scenario that actually
//! decides the spike: in real use, a user's component crate registers stories
//! and the storybook binary may never name them.

use s2_core::{StoryDef, STORIES};
use linkme::distributed_slice;

#[distributed_slice(STORIES)]
static BUTTON_PRIMARY: StoryDef = StoryDef {
    id: "forms-button--primary",
    title: "Forms/Button",
    name: "Primary",
    render: || 10,
};

#[distributed_slice(STORIES)]
static BUTTON_LOADING: StoryDef = StoryDef {
    id: "forms-button--loading",
    title: "Forms/Button",
    name: "Loading",
    render: || 20,
};

/// Deliberately the ONLY public item the binary will call. Everything above is
/// unreferenced by name, so if GC is going to bite, it bites here.
pub fn touch() -> u32 { 7 }
