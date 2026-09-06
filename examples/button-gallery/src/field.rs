//! A third component, chosen because it is the one that needs M2 to be
//! demonstrable at all.
//!
//! A `Button` shows off *controls*: edit a prop, watch the render change. A text
//! field shows off the two harder halves — an `EventHandler` carrying a real
//! payload into the actions panel, and a **controlled** input, whose value comes
//! from a prop and whose `oninput` has to push a new value back out or the field
//! will not accept a keystroke.

use dioxus::prelude::*;
use dioxus_storybook::prelude::*;

/// How the field reports its state.
#[derive(Clone, Copy, PartialEq, Debug, ControlEnum)]
pub enum FieldStatus {
    /// Nothing to report.
    Neutral,
    /// The value was accepted.
    Valid,
    /// The value was rejected.
    Invalid,
}

/// A labelled text field.
#[derive(Props, Clone, PartialEq, Controls)]
pub struct FieldProps {
    /// The label above the input.
    pub label: String,
    /// The current value. This is a *controlled* input: the field never keeps
    /// its own copy, so whoever owns this prop has to update it.
    pub value: String,
    /// Placeholder shown when the value is empty.
    pub placeholder: String,
    /// Validation state, which colours the border.
    pub status: FieldStatus,
    /// Whether the field rejects input.
    pub disabled: bool,
    /// Fired on every keystroke, with the new value.
    pub oninput: EventHandler<String>,
    /// Fired when the field loses focus.
    pub onblur: EventHandler<FocusEvent>,
}

#[component]
pub fn Field(props: FieldProps) -> Element {
    let border = match props.status {
        FieldStatus::Neutral => "#CFCEC7",
        FieldStatus::Valid => "#2F7D4F",
        FieldStatus::Invalid => "#9B1B1B",
    };
    let opacity = if props.disabled { 0.55 } else { 1.0 };
    let input_style = format!(
        "border:1px solid {border}; border-radius:6px; padding:7px 10px; \
         font:14px/1.3 ui-sans-serif,system-ui; min-width:220px; opacity:{opacity};"
    );

    // A real form control needs an id and a name, and a `<label for=..>` needs
    // something to point at. Derived from the label so the story stays a
    // one-liner; a real design system would take it as a prop.
    let field_id = format!("field-{}", dioxus_storybook::kebab(&props.label));

    rsx! {
        div { style: "display:flex; flex-direction:column; gap:5px;",
            label {
                r#for: "{field_id}",
                style: "font:500 12px ui-sans-serif,system-ui; color:#4A5058;",
                "{props.label}"
            }
            input {
                id: "{field_id}",
                name: "{field_id}",
                r#type: "text",
                style: "{input_style}",
                value: "{props.value}",
                placeholder: "{props.placeholder}",
                disabled: props.disabled,
                oninput: move |e| props.oninput.call(e.value()),
                onblur: move |e| props.onblur.call(e),
            }
        }
    }
}
