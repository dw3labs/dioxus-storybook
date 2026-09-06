//! Realistic design-system crate layout.

pub mod stories;   // declared, but nothing outside ever names it

/// The only thing the storybook binary actually calls — stands in for a
/// component the user imports.
pub fn touch() -> u32 { 7 }
