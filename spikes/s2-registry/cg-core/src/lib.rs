//! Codegen-based registry. No link-section or ctor magic: a build step emits
//! explicit `&'static` references, so every story is reachable from the crate
//! root and no linker decision can drop it.

use core::hint::black_box;

#[derive(Debug)]
pub struct StoryDef {
    pub id: &'static str,
    pub title: &'static str,
    pub name: &'static str,
    pub render: fn() -> u32,
}

/// Declares one story. The generated registry references `$ident` by path,
/// which is what makes it survive codegen-unit partitioning.
#[macro_export]
macro_rules! story {
    ($ident:ident, $id:expr, $title:expr, $name:expr, $render:expr) => {
        pub static $ident: $crate::StoryDef = $crate::StoryDef {
            id: $id, title: $title, name: $name, render: $render,
        };
    };
}

pub fn count(all: &[&'static StoryDef]) -> u32 {
    let mut n = 0u32;
    for s in all { black_box(s); n = n.wrapping_add(1); }
    black_box(n)
}
pub fn checksum(all: &[&'static StoryDef]) -> u32 {
    let mut a = 0u32;
    for s in all { a = a.wrapping_add(black_box((s.render)())); }
    black_box(a)
}
pub fn id_len_sum(all: &[&'static StoryDef]) -> u32 {
    let mut a = 0u32;
    for s in all { a = a.wrapping_add(black_box(s.id.len() as u32)); }
    black_box(a)
}
pub fn id_byte(all: &[&'static StoryDef], i: u32, j: u32) -> u32 {
    for (n, s) in all.iter().enumerate() {
        if n as u32 == black_box(i) {
            let b = s.id.as_bytes(); let j = black_box(j) as usize;
            return if j < b.len() { b[j] as u32 } else { 0 };
        }
    }
    u32::MAX
}
