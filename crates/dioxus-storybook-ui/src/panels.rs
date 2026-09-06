//! The addon panels: Controls and Actions.
//!
//! Both are manager-side. Neither touches the preview directly — the controls
//! panel reports an edit upward and the manager broadcasts it on the
//! [`Channel`], exactly as an out-of-process addon would have to. That is the
//! whole reason the panel does not simply hold the preview's args signal.
//!
//! # Where the numbers come from
//!
//! A control needs two things: the *shape* of a prop and its *current value*.
//! They arrive by different routes on purpose.
//!
//! - Shape ([`ArgType`]) is a compile-time constant, so the manager reads it
//!   straight off the [`StoryDef`](dioxus_storybook_core::StoryDef).
//! - Value comes from the preview, over the channel, as
//!   [`Event::StoryPrepared`](dioxus_storybook_core::Event::StoryPrepared).
//!   Evaluating a story's props needs a live Dioxus scope, and from M3 the
//!   story functions are not even in this bundle.
//!
//! [`Channel`]: dioxus_storybook_core::Channel

use dioxus::prelude::*;
use dioxus_storybook_core::{ArgMap, ArgType, ArgValue, Control};

/// One line in the actions log.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionEntry {
    /// Monotonic sequence number, used as the list key.
    pub seq: usize,
    /// The prop that was called, e.g. `onclick`.
    pub name: String,
    /// The payload, rendered by the generated wrapper.
    pub payload: String,
}

/// How many actions to keep. A component in a render loop can call a handler
/// thousands of times; the panel is a tail, not an archive.
pub const ACTION_LOG_CAP: usize = 200;

/// Which addon panel is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelTab {
    /// The controls table.
    Controls,
    /// The actions log.
    Actions,
}

/// The value a control should currently display: the override if the panel or
/// the URL set one, otherwise the story's own default.
fn effective(name: &str, overrides: &ArgMap, initial: &ArgMap) -> Option<ArgValue> {
    overrides.get(name).or_else(|| initial.get(name)).cloned()
}

/// The tabbed panel below the canvas.
#[component]
pub fn AddonPanel(
    arg_types: &'static [ArgType],
    initial: ArgMap,
    overrides: ArgMap,
    actions: Vec<ActionEntry>,
    tab: PanelTab,
    open: bool,
    on_tab: EventHandler<PanelTab>,
    on_toggle: EventHandler<()>,
    on_set: EventHandler<(String, ArgValue)>,
    on_unset: EventHandler<String>,
    on_reset: EventHandler<()>,
    on_clear: EventHandler<()>,
) -> Element {
    let controllable = arg_types.iter().filter(|a| a.is_dynamic()).count();
    let tab_class = |which: PanelTab| {
        if which == tab && open {
            "dxsb-tab active"
        } else {
            "dxsb-tab"
        }
    };

    rsx! {
        section { class: if open { "dxsb-panel open" } else { "dxsb-panel" },
            header { class: "dxsb-tabs",
                button {
                    class: tab_class(PanelTab::Controls),
                    onclick: move |_| {
                        if tab == PanelTab::Controls && open { on_toggle.call(()) } else { on_tab.call(PanelTab::Controls) }
                    },
                    "Controls"
                    if controllable > 0 {
                        span { class: "dxsb-count", "{controllable}" }
                    }
                }
                button {
                    class: tab_class(PanelTab::Actions),
                    onclick: move |_| {
                        if tab == PanelTab::Actions && open { on_toggle.call(()) } else { on_tab.call(PanelTab::Actions) }
                    },
                    "Actions"
                    if !actions.is_empty() {
                        span { class: "dxsb-count", "{actions.len()}" }
                    }
                }
                span { class: "dxsb-spacer" }
                if open {
                    match tab {
                        PanelTab::Controls => rsx! {
                            button {
                                class: "dxsb-panelbtn",
                                disabled: overrides.is_empty(),
                                onclick: move |_| on_reset.call(()),
                                "Reset to story defaults"
                            }
                        },
                        PanelTab::Actions => rsx! {
                            button {
                                class: "dxsb-panelbtn",
                                disabled: actions.is_empty(),
                                onclick: move |_| on_clear.call(()),
                                "Clear"
                            }
                        },
                    }
                }
                button {
                    class: "dxsb-panelbtn",
                    onclick: move |_| on_toggle.call(()),
                    title: if open { "Collapse panel" } else { "Expand panel" },
                    if open { "▾" } else { "▸" }
                }
            }
            if open {
                div { class: "dxsb-panelbody",
                    match tab {
                        PanelTab::Controls => rsx! {
                            ControlsTable { arg_types, initial, overrides, on_set, on_unset }
                        },
                        PanelTab::Actions => rsx! {
                            ActionsLog { actions }
                        },
                    }
                }
            }
        }
    }
}

/// One row per prop, in declaration order.
#[component]
fn ControlsTable(
    arg_types: &'static [ArgType],
    initial: ArgMap,
    overrides: ArgMap,
    on_set: EventHandler<(String, ArgValue)>,
    on_unset: EventHandler<String>,
) -> Element {
    if arg_types.is_empty() {
        return rsx! {
            p { class: "dxsb-panelempty",
                "This story has no props table. Stories that return "
                code { "Element" }
                " build their own markup, so there is nothing to introspect — return the component's "
                code { "Props" }
                " type instead to get controls."
            }
        };
    }

    rsx! {
        table { class: "dxsb-controls",
            thead {
                tr {
                    th { "Name" }
                    th { "Control" }
                    th { class: "dxsb-th-type", "Type" }
                    th { }
                }
            }
            tbody {
                for arg in arg_types.iter() {
                    ControlRow {
                        key: "{arg.name}",
                        arg: arg.clone(),
                        value: effective(arg.name, &overrides, &initial),
                        overridden: overrides.get(arg.name).is_some(),
                        on_set,
                        on_unset,
                    }
                }
            }
        }
    }
}

#[component]
fn ControlRow(
    arg: ArgType,
    value: Option<ArgValue>,
    overridden: bool,
    on_set: EventHandler<(String, ArgValue)>,
    on_unset: EventHandler<String>,
) -> Element {
    // `&'static str` is `Copy`, so each closure can take its own copy of the
    // name without the row having to clone a `String` per widget.
    let name = arg.name;
    // Every widget carries an id and a name. Not decoration: a form control
    // without either makes the browser complain, breaks `for`/`id` label
    // association, and — for radios — is what groups the options in the first
    // place. Prop names are unique within a props struct, so this is stable.
    let field_id = format!("dxsb-ctrl-{name}");
    // What the prop name in the first column should point `for` at — if
    // anything. A `for` naming an id that does not exist is worse than no
    // `for` at all: the browser flags it and assistive tech follows it nowhere.
    //
    // Two rows have no single control to point at. An `Action` or skipped prop
    // renders prose, not a widget; a radio group renders one input per option,
    // so the label targets the first, which is what clicking it should focus.
    let label_target: Option<String> = match &arg.control {
        Control::Action | Control::None => None,
        Control::Radio { options } => options.first().map(|o| format!("{field_id}-{o}")),
        _ => Some(field_id.clone()),
    };
    let text = value.as_ref().map(ArgValue::as_text).unwrap_or_default();
    let num = value.as_ref().and_then(ArgValue::as_num).unwrap_or(0.0);
    let flag = value.as_ref().and_then(ArgValue::as_bool).unwrap_or(false);
    let is_null = matches!(value, Some(ArgValue::Null) | None);

    let widget = match &arg.control {
        Control::Text => rsx! {
            input {
                id: "{field_id}",
                name: "{name}",
                class: "dxsb-input",
                r#type: "text",
                value: "{text}",
                oninput: move |e| on_set.call((name.to_string(), ArgValue::Text(e.value()))),
            }
        },
        Control::Toggle => rsx! {
            label { class: "dxsb-switch",
                input {
                    id: "{field_id}",
                    name: "{name}",
                    r#type: "checkbox",
                    checked: flag,
                    oninput: move |e| {
                        on_set.call((name.to_string(), ArgValue::Bool(e.value() == "true")))
                    },
                }
                span { if flag { "true" } else { "false" } }
            }
        },
        Control::Number { min, max, step } => {
            let min = min.map(|v| v.to_string()).unwrap_or_default();
            let max = max.map(|v| v.to_string()).unwrap_or_default();
            let step = step.map(|v| v.to_string()).unwrap_or_else(|| "any".into());
            rsx! {
                input {
                    id: "{field_id}",
                    name: "{name}",
                    class: "dxsb-input dxsb-num",
                    r#type: "number",
                    min: "{min}",
                    max: "{max}",
                    step: "{step}",
                    value: "{text}",
                    oninput: move |e| {
                        // A half-typed number ("-", "1e") is not an error, it is
                        // a keystroke. Ignore it and keep the last good value.
                        if let Ok(v) = e.value().parse::<f64>() {
                            on_set.call((name.to_string(), ArgValue::Num(v)));
                        }
                    },
                }
            }
        }
        Control::Range { min, max, step } => {
            let (min, max, step) = (*min, *max, *step);
            rsx! {
                div { class: "dxsb-rangewrap",
                    input {
                        id: "{field_id}",
                        name: "{name}",
                        class: "dxsb-range",
                        r#type: "range",
                        min: "{min}",
                        max: "{max}",
                        step: "{step}",
                        value: "{num}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<f64>() {
                                on_set.call((name.to_string(), ArgValue::Num(v)));
                            }
                        },
                    }
                    code { class: "dxsb-readout", "{text}" }
                }
            }
        }
        Control::Select { options } => rsx! {
            select {
                id: "{field_id}",
                name: "{name}",
                class: "dxsb-input",
                onchange: move |e| on_set.call((name.to_string(), ArgValue::Variant(e.value()))),
                for option in options.iter() {
                    option {
                        key: "{option}",
                        value: "{option}",
                        selected: text == **option,
                        "{option}"
                    }
                }
            }
        },
        Control::Radio { options } => rsx! {
            div { class: "dxsb-radios",
                for option in options.iter() {
                    label { key: "{option}", class: "dxsb-radio",
                        input {
                            id: "{field_id}-{option}",
                            r#type: "radio",
                            name: "{name}",
                            value: "{option}",
                            checked: text == **option,
                            onchange: move |_| {
                                on_set.call((name.to_string(), ArgValue::Variant(option.to_string())))
                            },
                        }
                        span { "{option}" }
                    }
                }
            }
        },
        Control::Color => {
            // `<input type="color">` will not show an empty value, so an unset
            // colour falls back to something neutral rather than rendering blank.
            let swatch = if text.starts_with('#') { text.clone() } else { "#000000".into() };
            rsx! {
                div { class: "dxsb-colorwrap",
                    input {
                        id: "{field_id}",
                        name: "{name}",
                        r#type: "color",
                        value: "{swatch}",
                        oninput: move |e| on_set.call((name.to_string(), ArgValue::Text(e.value()))),
                    }
                    input {
                        id: "{field_id}-hex",
                        name: "{name}-hex",
                        class: "dxsb-input dxsb-hex",
                        r#type: "text",
                        value: "{text}",
                        oninput: move |e| on_set.call((name.to_string(), ArgValue::Text(e.value()))),
                    }
                }
            }
        }
        Control::Action => rsx! {
            span { class: "dxsb-muted", "calls appear in Actions" }
        },
        Control::None => rsx! {
            span { class: "dxsb-muted", "not controllable" }
        },
        // `Control` is `#[non_exhaustive]` and lives in another crate, so this
        // arm is mandatory. It names the variant rather than rendering blank,
        // so a widget added to core and forgotten here is visible rather than
        // invisible.
        other => rsx! {
            span { class: "dxsb-muted", "unsupported control: {other:?}" }
        },
    };

    let optional = if arg.required { "" } else { " ?" };
    rsx! {
        tr { class: if overridden { "dxsb-ctrl overridden" } else { "dxsb-ctrl" },
            td { class: "dxsb-ctrlname",
                match &label_target {
                    Some(target) => rsx! {
                        label { class: "dxsb-propname", r#for: "{target}", "{arg.name}" }
                    },
                    None => rsx! {
                        span { class: "dxsb-propname", "{arg.name}" }
                    },
                }
                span { class: "dxsb-optional", "{optional}" }
                if !arg.docs.is_empty() {
                    span { class: "dxsb-propdocs", "{arg.docs}" }
                }
            }
            td { class: "dxsb-ctrlwidget", {widget} }
            td { class: "dxsb-ctrltype", code { "{arg.ty}" } }
            td { class: "dxsb-ctrlreset",
                if overridden {
                    button {
                        class: "dxsb-unset",
                        title: "Back to the story's value",
                        onclick: move |_| on_unset.call(name.to_string()),
                        "⟲"
                    }
                } else if is_null && arg.is_dynamic() {
                    span { class: "dxsb-muted", "unset" }
                }
            }
        }
    }
}

#[component]
fn ActionsLog(actions: Vec<ActionEntry>) -> Element {
    if actions.is_empty() {
        return rsx! {
            p { class: "dxsb-panelempty",
                "No handler calls yet. Every "
                code { "EventHandler" }
                " prop on a story is wrapped automatically — interact with the component above."
            }
        };
    }
    rsx! {
        ol { class: "dxsb-actions",
            // Newest first: the interesting call is the one that just happened.
            for entry in actions.iter().rev() {
                li { key: "{entry.seq}",
                    span { class: "dxsb-actionname", "{entry.name}" }
                    code { class: "dxsb-actionpayload", "{entry.payload}" }
                }
            }
        }
    }
}
