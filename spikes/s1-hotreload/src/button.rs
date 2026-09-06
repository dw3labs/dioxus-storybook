use dioxus::prelude::*;
use s3_macro::{ControlEnum, Controls};

#[derive(Clone, Copy, PartialEq, Debug, ControlEnum)]
pub enum ButtonVariant { Primary, Secondary, Ghost, Danger }

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
        ButtonVariant::Primary   => ("#C4451B", "#fff",    "#C4451B"),
        ButtonVariant::Secondary => ("#fff",    "#17191C", "#CFCEC7"),
        ButtonVariant::Ghost     => ("transparent", "#C4451B", "transparent"),
        ButtonVariant::Danger    => ("#9B1B1B", "#fff",    "#9B1B1B"),
    };
    // rsx! format strings only interpolate idents, so derive everything first.
    let font_px = 14.0 * props.scale;
    let pad_y = 8.0 * props.scale;
    let pad_x = 16.0 * props.scale;
    let radius = 6.0 * props.scale;
    let opacity = if props.disabled { 0.5 } else { 1.0 };
    let cursor = if props.disabled { "not-allowed" } else { "pointer" };
    let tooltip = props.tooltip.clone().unwrap_or_default();
    let style = format!(
        "background:{bg}; color:{fg}; border:1px solid {border}; \
         font:500 {font_px}px/1.2 ui-sans-serif,system-ui; \
         padding:{pad_y}px {pad_x}px; border-radius:{radius}px; \
         opacity:{opacity}; cursor:{cursor};"
    );
    rsx! {
        button {
            disabled: props.disabled,
            title: "{tooltip}",
            onclick: move |e| props.onclick.call(e),
            style: "{style}",
            "{props.label}"
        }
    }
}
