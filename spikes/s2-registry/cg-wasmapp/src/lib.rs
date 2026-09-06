use s2_cg_core::{StoryDef, story};

pub mod local { use s2_cg_core::story; story!(APP_LOCAL, "app--local", "App", "Local", || 100); }
story!(CORE_LOCAL, "core--local", "Core", "Local", || 1);

fn all() -> Vec<&'static StoryDef> {
    let mut v: Vec<&'static StoryDef> = vec![&CORE_LOCAL, &local::APP_LOCAL];
    v.extend_from_slice(s2_cg_components::STORIES);
    v
}

#[unsafe(no_mangle)]
pub extern "C" fn story_count() -> u32 { s2_cg_core::count(&all()) }
#[unsafe(no_mangle)]
pub extern "C" fn story_checksum() -> u32 { s2_cg_core::checksum(&all()) }
#[unsafe(no_mangle)]
pub extern "C" fn story_id_len_sum() -> u32 { s2_cg_core::id_len_sum(&all()) }
#[unsafe(no_mangle)]
pub extern "C" fn story_id_byte(i: u32, j: u32) -> u32 { s2_cg_core::id_byte(&all(), i, j) }
