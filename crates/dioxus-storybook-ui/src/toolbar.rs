//! The toolbar addons: the Canvas/Docs toggle, the globals bar and the viewport
//! picker.
//!
//! Manager-side, like every other addon surface: both edit the manager's
//! globals signal, which goes out on the [`Channel`] as
//! [`Event::SetGlobals`](dioxus_storybook_core::Event::SetGlobals). Neither
//! touches the preview.
//!
//! The viewport is the one that needs the least of the preview: the frame's
//! width belongs to the shell, so a viewport change is a style attribute, not a
//! message. It rides along in `SetGlobals` anyway — the story is entitled to
//! know what size it was put at, and the URL is entitled to carry it.
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
use dioxus_storybook_core::viewport::RESPONSIVE;
use dioxus_storybook_core::{ArgMap, ArgValue, Control, GlobalType, Viewport, ViewportSelection};

/// The Canvas / Docs toggle, or nothing when this component has no docs page.
///
/// It is a pair of buttons and not a link, but what it actually does is change
/// the selected **id**: a docs page is an entry in the same id space a story is,
/// so switching to it goes through the same signal, the same URL parameter and
/// the same wire message as walking the sidebar. That is the whole reason there
/// is no `viewMode=docs` here to match Storybook's — the id already says it.
#[component]
pub fn DocsToggle(
    /// The docs entry id, or `None` when this component has no docs page.
    docs_id: Option<String>,
    /// The story the Canvas button lands on, or `None` when the component has
    /// none — which cannot happen for a resolved docs page, but the signature
    /// should not have to promise that.
    canvas_id: Option<String>,
    /// Whether the docs page is what is currently showing.
    on_docs: bool,
    on_pick: EventHandler<String>,
) -> Element {
    let (Some(docs_id), Some(canvas_id)) = (docs_id, canvas_id) else {
        return rsx! {};
    };
    rsx! {
        div { class: "dxsb-docstoggle", role: "tablist",
            button {
                class: if on_docs { "dxsb-docstab" } else { "dxsb-docstab active" },
                role: "tab",
                aria_selected: !on_docs,
                onclick: move |_| on_pick.call(canvas_id.clone()),
                "Canvas"
            }
            button {
                class: if on_docs { "dxsb-docstab active" } else { "dxsb-docstab" },
                role: "tab",
                aria_selected: on_docs,
                onclick: move |_| on_pick.call(docs_id.clone()),
                "Docs"
            }
        }
    }
}

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

/// The viewport picker: the size of the frame, and which way up.
///
/// The one addon that needs nothing from the preview. A viewport *is* the
/// frame's width, and the shell owns the frame — so this component picks a
/// number and the story finds out the way a real page does, by being that
/// size. Nothing is measured, nothing is injected, and the story's own media
/// queries do the rest.
///
/// It is rendered next to [`GlobalsBar`] and not inside it because the
/// selection is not a *declared* global: the project does not list it, the
/// widget is not one of the [`Control`] widgets, and it carries a second
/// control — rotation — that no declared global has.
#[component]
pub fn ViewportPicker(
    available: &'static [Viewport],
    /// The size in force, or `None` for a responsive canvas.
    selection: Option<ViewportSelection>,
    on_pick: EventHandler<String>,
    on_rotate: EventHandler<()>,
) -> Element {
    if available.is_empty() {
        return rsx! {};
    }
    let current = selection.map(|s| s.name()).unwrap_or(RESPONSIVE);
    let field_id = "dxsb-viewport";
    rsx! {
        div { class: "dxsb-viewport",
            label { class: "dxsb-global", r#for: "{field_id}", title: "The size of the story canvas",
                span { class: "dxsb-global-title", "Viewport" }
                select {
                    id: "{field_id}",
                    name: "viewport",
                    class: "dxsb-globals-input",
                    onchange: move |e| on_pick.call(e.value()),
                    option {
                        value: "{RESPONSIVE}",
                        selected: current == RESPONSIVE,
                        "Responsive"
                    }
                    for view in available.iter() {
                        option {
                            key: "{view.name()}",
                            value: "{view.name()}",
                            selected: current == view.name(),
                            "{view.title()} — {view.width()}×{view.height()}"
                        }
                    }
                }
            }
            // Rotation is meaningless without a fixed size, so the button is
            // present-but-disabled rather than appearing and disappearing: a
            // control that moves the ones next to it is worse than a dim one.
            button {
                class: "dxsb-viewport-rotate",
                title: "Turn the canvas on its side",
                disabled: selection.is_none(),
                onclick: move |_| on_rotate.call(()),
                "⟳"
            }
            if let Some(selection) = selection {
                // Read off the selection, not off the viewport: under rotation
                // these are the declared numbers the other way round, and the
                // dropdown still shows them unrotated.
                span { class: "dxsb-viewport-size",
                    "{selection.width()}×{selection.height()}"
                }
            }
        }
    }
}
