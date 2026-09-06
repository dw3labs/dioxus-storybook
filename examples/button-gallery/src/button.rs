//! The component under test. Nothing here knows about storybook except the two
//! extra derives.

use dioxus::prelude::*;
use dioxus_storybook::prelude::*;

/// How much visual weight a button carries.
#[derive(Clone, Copy, PartialEq, Debug, ControlEnum)]
pub enum ButtonVariant {
    /// The primary call to action.
    Primary,
    /// A secondary, outlined action.
    Secondary,
    /// A borderless, text-only action.
    Ghost,
    /// A destructive action.
    Danger,
}

/// A button.
#[derive(Props, Clone, PartialEq, Controls)]
pub struct ButtonProps {
    /// Text shown inside the button.
    pub label: String,
    /// Visual emphasis of the button.
    pub variant: ButtonVariant,
    /// Relative size multiplier.
    #[control(range(min = 0.75, max = 2.5, step = 0.05))]
    pub scale: f32,
    /// Whether the button rejects interaction.
    pub disabled: bool,
    /// Optional tooltip shown on hover.
    pub tooltip: Option<String>,
    /// Fired on click.
    pub onclick: EventHandler<MouseEvent>,
}

#[component]
pub fn Button(props: ButtonProps) -> Element {
    let (bg, fg, border) = match props.variant {
        ButtonVariant::Primary => ("#C4451B", "#fff", "#C4451B"),
        ButtonVariant::Secondary => ("#fff", "#17191C", "#CFCEC7"),
        ButtonVariant::Ghost => ("transparent", "#C4451B", "transparent"),
        ButtonVariant::Danger => ("#9B1B1B", "#fff", "#9B1B1B"),
    };
    // STANDING FACT: rsx! format strings interpolate idents only, so anything
    // computed has to land in a `let` first.
    let font_px = 14.0 * props.scale;
    let pad_y = 8.0 * props.scale;
    let pad_x = 16.0 * props.scale;
    let radius = 6.0 * props.scale;
    let opacity = if props.disabled { 0.5 } else { 1.0 };
    let cursor = if props.disabled { "not-allowed" } else { "pointer" };
    let style = format!(
        "background:{bg}; color:{fg}; border:1px solid {border}; \
         font:500 {font_px}px/1.2 ui-sans-serif,system-ui; \
         padding:{pad_y}px {pad_x}px; border-radius:{radius}px; \
         opacity:{opacity}; cursor:{cursor};"
    );
    rsx! {
        style { {TOOLTIP_CSS} }
        span { class: "bg-tipwrap",
            button {
                disabled: props.disabled,
                onclick: move |e| props.onclick.call(e),
                style: "{style}",
                "{props.label}"
            }
            // Not the native `title` attribute, for two reasons that both show
            // up the moment you drive this from the controls panel: browsers
            // delay a native tooltip by a second or more, so editing the prop
            // looks like it did nothing; and they suppress it entirely on a
            // disabled control, so the `Disabled` story could never show one.
            // Hovering the wrapper rather than the button is what makes the
            // second case work.
            if let Some(tip) = props.tooltip.clone() {
                span { class: "bg-tip", role: "tooltip", "{tip}" }
            }
        }
    }
}

/// Styles for the tooltip bubble.
///
/// Carried by the component rather than the manager: a story's component owns
/// its own presentation, and from M3 the preview is a separate document that
/// the manager's stylesheet deliberately cannot reach into.
const TOOLTIP_CSS: &str = r#"
.bg-tipwrap{position:relative;display:inline-block}
.bg-tip{position:absolute;bottom:calc(100% + 7px);left:50%;transform:translateX(-50%);
  background:#17191C;color:#fff;font:12px/1.35 ui-sans-serif,system-ui;white-space:nowrap;
  padding:4px 8px;border-radius:5px;opacity:0;visibility:hidden;transition:opacity .09s;
  pointer-events:none;z-index:10}
.bg-tip::after{content:"";position:absolute;top:100%;left:50%;transform:translateX(-50%);
  border:4px solid transparent;border-top-color:#17191C}
.bg-tipwrap:hover .bg-tip{opacity:1;visibility:visible}
"#;
