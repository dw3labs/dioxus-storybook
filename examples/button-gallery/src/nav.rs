//! A component whose whole point is that it changes shape with the canvas.
//!
//! Everything else in this gallery looks the same at any width, which makes the
//! viewport picker look like a picture frame. This one has a media query in it,
//! so dragging the toolbar from Desktop to Mobile is the only way to see half
//! of its behaviour — which is the argument for the addon in one component.

use dioxus::prelude::*;
use dioxus_storybook::prelude::*;

/// The component's own stylesheet, carried with it.
///
/// It has to be: the story renders in the preview iframe, which inherits
/// nothing from the shell. A component that brings its own styles works there
/// with no help; one that relies on an app-wide stylesheet needs a decorator to
/// put that stylesheet in the frame.
const NAV_CSS: &str = r#"
.demo-nav{display:flex;align-items:center;gap:20px;width:100%;padding:12px 20px;
  /* The gallery's project decorator centres every story in the canvas, which is
     right for a button and wrong for a site header — on a 1112px tablet it puts
     the navbar halfway down the page. A component that belongs at the top of a
     document says so itself. */
  align-self:start;
  border-bottom:1px solid #E2E1DB;background:inherit;color:inherit;
  font:14px ui-sans-serif,system-ui,-apple-system,sans-serif}
.demo-nav-brand{font-weight:600;white-space:nowrap}
.demo-nav-links{display:flex;gap:18px;flex:1;list-style:none;margin:0;padding:0}
.demo-nav-links li{white-space:nowrap;opacity:.75}
.demo-nav-cta{background:#C4451B;color:#fff;border:0;border-radius:6px;
  padding:6px 13px;font:inherit;cursor:pointer;white-space:nowrap}
.demo-nav-burger{display:none;background:none;border:0;font-size:19px;
  line-height:1;cursor:pointer;color:inherit}
@media (max-width:640px){
  .demo-nav{justify-content:space-between}
  .demo-nav-links{display:none}
  .demo-nav-cta{display:none}
  .demo-nav-burger{display:block}
}
"#;

/// A site header that collapses to a menu button under 640px.
#[derive(Props, Clone, PartialEq, Controls)]
pub struct NavbarProps {
    /// The wordmark on the left.
    pub brand: String,
    /// The links in the middle. Hidden below the breakpoint.
    pub links: Vec<String>,
    /// The call to action on the right. Hidden below the breakpoint.
    pub cta: String,
    /// Fired when the collapsed menu button is pressed.
    pub onmenu: EventHandler<()>,
}

#[component]
pub fn Navbar(props: NavbarProps) -> Element {
    rsx! {
        style { {NAV_CSS} }
        nav { class: "demo-nav",
            span { class: "demo-nav-brand", "{props.brand}" }
            ul { class: "demo-nav-links",
                for link in props.links.iter() {
                    li { key: "{link}", "{link}" }
                }
            }
            button { class: "demo-nav-cta", "{props.cta}" }
            button {
                class: "demo-nav-burger",
                aria_label: "Open the menu",
                onclick: move |_| props.onmenu.call(()),
                "☰"
            }
        }
    }
}
