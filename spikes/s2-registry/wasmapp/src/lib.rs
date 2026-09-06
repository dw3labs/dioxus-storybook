use s2_core::{StoryDef, STORIES};
use linkme::distributed_slice;

/// Registered in the binary/entry crate itself.
#[distributed_slice(STORIES)]
static APP_LOCAL: StoryDef = StoryDef {
    id: "app--local",
    title: "App",
    name: "Local",
    render: || 100,
};

#[unsafe(no_mangle)]
pub extern "C" fn story_count() -> u32 { s2_core::count() }

#[unsafe(no_mangle)]
pub extern "C" fn story_checksum() -> u32 { s2_core::checksum() }

/// Referenced so `s2-components` is definitely linked in at all.
#[unsafe(no_mangle)]
pub extern "C" fn touch_components() -> u32 { s2_components::touch() }
