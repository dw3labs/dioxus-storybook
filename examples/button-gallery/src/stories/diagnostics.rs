//! A story that deliberately breaks, to show what happens when one does.
//!
//! From M3 the canvas is an iframe, so a panicking story kills *that* document
//! and leaves the shell standing. Without a report the workbench would look
//! healthy next to a rectangle that had quietly stopped, so the preview installs
//! a panic hook and tells the manager on its way out.
//!
//! Flip `boom` on in the controls panel to see it: the frame dies, the shell
//! shows the panic message, and "Reload the preview" brings it back. Nothing
//! recovers on its own — wasm here is built `panic = "abort"`, so the module is
//! gone rather than unwound.

use dioxus_storybook::prelude::*;

/// A component that panics on request.
#[derive(Props, Clone, PartialEq, Controls)]
pub struct CrashProps {
    /// Turn this on to panic the preview document.
    pub boom: bool,
}

#[component]
pub fn Crash(props: CrashProps) -> Element {
    assert!(!props.boom, "the Crash story was asked to panic, and obliged");
    rsx! {
        p { style: "color:#4A5058;max-width:40ch;text-align:center;line-height:1.6",
            "This story panics when you turn "
            code { "boom" }
            " on. The shell survives; the frame does not."
        }
    }
}

story_meta! {
    title: "Diagnostics/Crash",
    component: Crash,
}

#[story]
fn on_demand() -> CrashProps {
    CrashProps { boom: false }
}
