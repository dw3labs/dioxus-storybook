//! Same spike, `inventory` instead of `linkme`.
//! inventory uses life-before-main constructors rather than link sections.
//!
//! NOTE: every accessor below is deliberately fold-resistant. A naive
//! `count()` gets const-folded to a literal by LTO + opt-level=z, which makes
//! the test pass without ever exercising the ctor mechanism. `black_box`
//! forces the registry to actually be walked at runtime.

use core::hint::black_box;

#[derive(Debug)]
pub struct StoryDef {
    pub id: &'static str,
    pub title: &'static str,
    pub name: &'static str,
    pub render: fn() -> u32,
}

inventory::collect!(StoryDef);

/// Walks the registry with an opaque barrier on each entry.
pub fn count() -> u32 {
    let mut n = 0u32;
    for s in inventory::iter::<StoryDef> {
        black_box(s);
        n = n.wrapping_add(1);
    }
    black_box(n)
}

pub fn checksum() -> u32 {
    let mut acc = 0u32;
    for s in inventory::iter::<StoryDef> {
        acc = acc.wrapping_add(black_box((s.render)()));
    }
    black_box(acc)
}

/// Indexes real string data out of the registry. Impossible to satisfy without
/// the actual entries being present in memory at runtime.
pub fn id_byte(i: u32, j: u32) -> u32 {
    for (n, s) in inventory::iter::<StoryDef>.into_iter().enumerate() {
        if n as u32 == black_box(i) {
            let b = s.id.as_bytes();
            let j = black_box(j) as usize;
            return if j < b.len() { b[j] as u32 } else { 0 };
        }
    }
    u32::MAX
}

/// Sum of every id length — another way to force the strings to exist.
pub fn id_len_sum() -> u32 {
    let mut acc = 0u32;
    for s in inventory::iter::<StoryDef> {
        acc = acc.wrapping_add(black_box(s.id.len() as u32));
    }
    black_box(acc)
}

inventory::submit! {
    StoryDef { id: "core--local", title: "Core", name: "Local", render: || 1 }
}
