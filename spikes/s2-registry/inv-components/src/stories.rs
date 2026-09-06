//! Stories in their own module. Nothing outside this file references any
//! symbol declared here — exactly how `button.stories.rs` will look.

use s2_inv_core::StoryDef;

inventory::submit! {
    StoryDef { id: "forms-button--primary", title: "Forms/Button", name: "Primary", render: || 10 }
}
inventory::submit! {
    StoryDef { id: "forms-button--loading", title: "Forms/Button", name: "Loading", render: || 20 }
}
