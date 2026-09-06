//! The globals toolbar.
//!
//! Manager-side, like every other addon surface: it edits the manager's globals
//! signal, which goes out on the [`Channel`] as
//! [`Event::SetGlobals`](dioxus_storybook_core::Event::SetGlobals). It never
//! touches the preview.
//!
//! # Why it is not the controls panel
//!
//! A global is declared with the same [`Control`] vocabulary a prop is, so the
//! widgets are the same idea. What differs is everything around them: a global
//! has no props table to sit in, no per-row reset, no docs column, and it has to
//! fit on one line of a toolbar that also holds a breadcrumb. So the widgets are
//! restated compactly here rather than the three-column
//! [`ControlRow`](crate::panels) being bent into a shape it was not built for.
//!
//! [`Channel`]: dioxus_storybook_core::Channel

use dioxus::prelude::*;
use dioxus_storybook_core::{ArgMap, ArgValue, Control, GlobalType};

/// One control per declared global, or nothing at all when none are declared.
#[component]
pub fn GlobalsBar(
    declared: &'static [GlobalType],
    /// The resolved set — declared defaults with the selections laid over.
    values: ArgMap,
    on_set: EventHandler<(String, ArgValue)>,
    on_reset: EventHandler<()>,
    /// Whether anything has been moved off its default, which is the only time
    /// "reset" means something.
    changed: bool,
) -> Element {
    if declared.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "dxsb-globals",
            for global in declared.iter() {
                GlobalPicker {
                    key: "{global.name()}",
                    global: global,
                    value: values.get(global.name()).cloned(),
                    on_set,
                }
            }
            button {
                class: "dxsb-globals-reset",
                title: "Put every global back to its declared default",
                disabled: !changed,
                onclick: move |_| on_reset.call(()),
                "⟲"
            }
        }
    }
}

#[component]
fn GlobalPicker(
    global: &'static GlobalType,
    value: Option<ArgValue>,
    on_set: EventHandler<(String, ArgValue)>,
) -> Element {
    let name = global.name();
    // Ids are needed for the same reason the controls panel needs them: a
    // `<label for>` pointing nowhere is worse than no label at all, and a radio
    // group is grouped by its `name` attribute or not at all.
    let field_id = format!("dxsb-global-{name}");
    let text = value.as_ref().map(ArgValue::as_text).unwrap_or_default();
    let flag = value.as_ref().and_then(ArgValue::as_bool).unwrap_or(false);
    let num = value.as_ref().and_then(ArgValue::as_num).unwrap_or(0.0);
    let hint = if global.description().is_empty() {
        global.title().to_string()
    } else {
        format!("{}: {}", global.title(), global.description())
    };

    let widget = match global.control() {
        Control::Select { options } | Control::Radio { options } => rsx! {
            // A radio group does not fit a toolbar, so a `Radio` global gets the
            // same dropdown a `Select` does. The declaration is about the value
            // being one of a fixed set; which of the two shapes renders it is a
            // decision the surface gets to make.
            select {
                id: "{field_id}",
                name: "{name}",
                class: "dxsb-globals-input",
                onchange: move |e| on_set.call((name.to_string(), ArgValue::Variant(e.value()))),
                for option in options.iter() {
                    option { key: "{option}", value: "{option}", selected: text == **option, "{option}" }
                }
            }
        },
        Control::Toggle => rsx! {
            input {
                id: "{field_id}",
                name: "{name}",
                r#type: "checkbox",
                checked: flag,
                onchange: move |e| {
                    on_set.call((name.to_string(), ArgValue::Bool(e.value() == "true")))
                },
            }
        },
        Control::Number { min, max, step } => {
            let (min, max, step) = (*min, *max, *step);
            rsx! {
                input {
                    id: "{field_id}",
                    name: "{name}",
                    class: "dxsb-globals-input narrow",
                    r#type: "number",
                    value: "{num}",
                    min: min.map(|v| v.to_string()),
                    max: max.map(|v| v.to_string()),
                    step: step.map(|v| v.to_string()),
                    oninput: move |e| {
                        if let Ok(n) = e.value().parse::<f64>() {
                            on_set.call((name.to_string(), ArgValue::Num(n)));
                        }
                    },
                }
            }
        }
        Control::Range { min, max, step } => {
            let (min, max, step) = (*min, *max, *step);
            rsx! {
                input {
                    id: "{field_id}",
                    name: "{name}",
                    class: "dxsb-globals-range",
                    r#type: "range",
                    value: "{num}",
                    min: "{min}",
                    max: "{max}",
                    step: "{step}",
                    oninput: move |e| {
                        if let Ok(n) = e.value().parse::<f64>() {
                            on_set.call((name.to_string(), ArgValue::Num(n)));
                        }
                    },
                }
            }
        }
        // Text, Color, and the two that make no sense for a global but must
        // still render something rather than vanish.
        _ => rsx! {
            input {
                id: "{field_id}",
                name: "{name}",
                class: "dxsb-globals-input",
                r#type: "text",
                value: "{text}",
                oninput: move |e| on_set.call((name.to_string(), ArgValue::Text(e.value()))),
            }
        },
    };

    rsx! {
        label { class: "dxsb-global", r#for: "{field_id}", title: "{hint}",
            span { class: "dxsb-global-title", "{global.title()}" }
            {widget}
        }
    }
}
