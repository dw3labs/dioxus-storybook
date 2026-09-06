use s2_inv_core::StoryDef;

inventory::submit! {
    StoryDef { id: "app--local", title: "App", name: "Local", render: || 100 }
}

#[unsafe(no_mangle)]
pub extern "C" fn story_count() -> u32 { s2_inv_core::count() }

#[unsafe(no_mangle)]
pub extern "C" fn story_checksum() -> u32 { s2_inv_core::checksum() }

#[unsafe(no_mangle)]
pub extern "C" fn story_id_byte(i: u32, j: u32) -> u32 { s2_inv_core::id_byte(i, j) }

#[unsafe(no_mangle)]
pub extern "C" fn story_id_len_sum() -> u32 { s2_inv_core::id_len_sum() }

#[unsafe(no_mangle)]
pub extern "C" fn touch_components() -> u32 { s2_inv_components::touch() }
