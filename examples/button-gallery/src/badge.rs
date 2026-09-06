//! A second component, so the sidebar has a tree to draw rather than a list.

use dioxus::prelude::*;
use dioxus_storybook::prelude::*;

/// A small status pill.
#[derive(Props, Clone, PartialEq, Controls)]
pub struct BadgeProps {
    /// The text inside the pill.
    pub text: String,
    /// Background colour, as any CSS colour.
    #[control(color)]
    pub color: String,
    /// Render the pill as a hollow outline instead of a solid fill.
    pub outline: bool,
}

#[component]
pub fn Badge(props: BadgeProps) -> Element {
    let color = props.color.clone();
    let style = if props.outline {
        format!("color:{color}; border:1px solid {color}; background:transparent;")
    } else {
        format!("color:#fff; border:1px solid {color}; background:{color};")
    };
    rsx! {
        span {
            style: "{style} font:600 11px ui-sans-serif,system-ui; letter-spacing:.04em; text-transform:uppercase; padding:3px 9px; border-radius:999px;",
            "{props.text}"
        }
    }
}
