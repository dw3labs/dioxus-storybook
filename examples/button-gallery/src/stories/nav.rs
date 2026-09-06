//! Stories for `Navbar` — the viewport addon, from the story author's side.
//!
//! The picker in the toolbar is book-wide state: pick Mobile and every story
//! you visit next is on a phone. What a *story* gets to say is where it opens,
//! and it says it with a parameter — the same three-level mechanism `layout`
//! uses, merged innermost-first.
//!
//! Note which half is which. The list of sizes on offer is a declaration and
//! lives on the `Project`; the size a story wants is `parameters { viewport }`;
//! the size you actually picked is a global, and rides in the URL next to the
//! theme. Nothing here is a fourth vocabulary.

use dioxus_storybook::prelude::*;

use crate::nav::{Navbar, NavbarProps};

story_meta! {
    title: "Layout/Navbar",
    component: Navbar,
    description: "A site header that collapses below 640px. Its docs page is a \
                  responsive canvas whatever the viewport picker says — the \
                  examples are laid out for reading, not for measuring.",
}

fn base() -> NavbarProps {
    NavbarProps {
        brand: "Acme".into(),
        links: vec!["Product".into(), "Pricing".into(), "Docs".into(), "Blog".into()],
        cta: "Get started".into(),
        onmenu: EventHandler::new(|_| {}),
    }
}

/// The full header. Drag the toolbar's viewport down to Mobile and watch the
/// links and the button give way to a menu icon — nothing re-rendered them and
/// nothing was told the width. The story is *in* a 360px document, so its own
/// media query fired, exactly as it will in your app.
#[story]
fn wide() -> NavbarProps {
    base()
}

/// The collapsed header, which opens on a phone without anyone touching the
/// toolbar first.
///
/// That is the whole of `parameters { viewport }`: a story whose subject only
/// exists below a breakpoint should not need a second instruction to be seen.
/// The toolbar still wins if you pick something — including *Responsive*, which
/// is a real choice here rather than the absence of one.
#[story(name = "On A Phone", parameters { viewport: "mobile" })]
fn on_a_phone() -> NavbarProps {
    base()
}
