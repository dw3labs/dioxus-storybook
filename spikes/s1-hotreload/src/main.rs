//! S1 spike: does the Storybook loop survive `dx serve` hot-patching?
//!
//! M0 exit criterion: a story renders, one control edits one prop live, and a
//! source edit hot-patches in under two seconds without losing the selected
//! story or the edited args.

mod button;
mod registry;
mod stories;

use dioxus::prelude::*;
use s3_core::{ArgMap, ArgValue, Control};
use stories::STORIES;

fn main() {
    dioxus::launch(App);
}

// ---- minimal Actions panel, to prove the EventHandler path works ----
static ACTIONS: GlobalSignal<Vec<String>> = Signal::global(Vec::new);

pub fn log_action(name: &str) {
    let n = ACTIONS.read().len();
    ACTIONS.write().insert(0, format!("{:>3}  {}", n + 1, name));
}

fn num(m: &ArgMap, k: &str) -> f64 {
    match m.get(k) { Some(ArgValue::Num(n)) => *n, _ => 0.0 }
}
fn text(m: &ArgMap, k: &str) -> String {
    match m.get(k) {
        Some(ArgValue::Text(s)) | Some(ArgValue::Variant(s)) => s.clone(),
        Some(ArgValue::Num(n)) => n.to_string(),
        _ => String::new(),
    }
}
fn flag(m: &ArgMap, k: &str) -> bool {
    matches!(m.get(k), Some(ArgValue::Bool(true)))
}

#[component]
fn App() -> Element {
    let mut selected = use_signal(|| STORIES[0].id);
    let mut args = use_signal(|| (STORIES[0].base_args)());

    let story = STORIES.iter().find(|s| s.id == selected()).copied().unwrap_or(STORIES[0]);
    let types = (story.arg_types)();

    rsx! {
        style { {CSS} }
        div { class: "app",

            // ---------------- sidebar ----------------
            aside { class: "sidebar",
                div { class: "brand", "dx-story " span { class: "tag", "S1" } }
                div { class: "group", "{story.title}" }
                for s in STORIES.iter() {
                    button {
                        key: "{s.id}",
                        class: if s.id == selected() { "item sel" } else { "item" },
                        onclick: move |_| { selected.set(s.id); args.set((s.base_args)()); },
                        "{s.name}"
                    }
                }
                div { class: "hint",
                    "Edit src/button.rs or src/stories.rs while dx serve is running. "
                    "Selection and args below must survive the patch."
                }
            }

            // ---------------- preview + panels ----------------
            main { class: "main",
                header { class: "toolbar",
                    span { class: "crumb", "{story.title} / {story.name}" }
                    span { class: "spacer" }
                    span { class: "id", "{story.id}" }
                }

                section { class: "canvas", { (story.render)(&args.read()) } }

                section { class: "panels",
                    div { class: "panel",
                        h3 { "Controls" }
                        table { class: "controls",
                            for at in types.iter() {
                                {
                                    let key = at.name.to_string();
                                    let m = args.read().clone();
                                    rsx! {
                                        tr { key: "{at.name}",
                                            th {
                                                div { class: "nm", "{at.name}" }
                                                div { class: "ty", "{at.ty}" }
                                                if !at.docs.is_empty() {
                                                    div { class: "doc", "{at.docs}" }
                                                }
                                            }
                                            td {
                                                match &at.control {
                                                    Control::Text => rsx! {
                                                        input { r#type: "text", value: "{text(&m, &key)}",
                                                            oninput: move |e| { let k = key.clone();
                                                                args.write().0.insert(k, ArgValue::Text(e.value())); } }
                                                    },
                                                    Control::Toggle => rsx! {
                                                        input { r#type: "checkbox", checked: flag(&m, &key),
                                                            oninput: move |e| { let k = key.clone();
                                                                args.write().0.insert(k, ArgValue::Bool(e.value() == "true")); } }
                                                    },
                                                    Control::Range { min, max, step } => rsx! {
                                                        div { class: "row",
                                                            input { r#type: "range",
                                                                min: "{min}", max: "{max}", step: "{step}",
                                                                value: "{num(&m, &key)}",
                                                                oninput: move |e| { let k = key.clone();
                                                                    if let Ok(v) = e.value().parse::<f64>() {
                                                                        args.write().0.insert(k, ArgValue::Num(v)); } } }
                                                            code { "{num(&m, &key):.2}" }
                                                        }
                                                    },
                                                    Control::Number { .. } => rsx! {
                                                        input { r#type: "number", value: "{num(&m, &key)}",
                                                            oninput: move |e| { let k = key.clone();
                                                                if let Ok(v) = e.value().parse::<f64>() {
                                                                    args.write().0.insert(k, ArgValue::Num(v)); } } }
                                                    },
                                                    Control::Select { options } | Control::Radio { options } => rsx! {
                                                        select {
                                                            onchange: move |e| { let k = key.clone();
                                                                args.write().0.insert(k, ArgValue::Variant(e.value())); },
                                                            for o in options.iter() {
                                                                option { key: "{o}", value: "{o}",
                                                                    selected: text(&m, &key) == **o, "{o}" }
                                                            }
                                                        }
                                                    },
                                                    Control::Color => rsx! {
                                                        input { r#type: "color", value: "{text(&m, &key)}",
                                                            oninput: move |e| { let k = key.clone();
                                                                args.write().0.insert(k, ArgValue::Text(e.value())); } }
                                                    },
                                                    Control::Action => rsx! { span { class: "muted", "logged to Actions" } },
                                                    Control::None => rsx! { span { class: "muted", "not controllable" } },
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        button { class: "reset",
                            onclick: move |_| args.set((story.base_args)()),
                            "Reset to story defaults"
                        }
                    }

                    div { class: "panel",
                        h3 { "Actions" }
                        if ACTIONS.read().is_empty() {
                            p { class: "muted", "Click the button in the canvas." }
                        }
                        ul { class: "actions",
                            for (i, a) in ACTIONS.read().iter().enumerate() {
                                li { key: "{i}", "{a}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

const CSS: &str = r#"
*{box-sizing:border-box} body{margin:0;font:14px/1.5 ui-sans-serif,system-ui;color:#17191C;background:#FAFAF8}
.app{display:grid;grid-template-columns:230px 1fr;height:100vh}
.sidebar{border-right:1px solid #E2E1DB;background:#F2F2EF;padding:14px;display:flex;flex-direction:column;gap:2px}
.brand{font:600 13px ui-monospace,monospace;letter-spacing:.06em;margin-bottom:14px}
.tag{background:#C4451B;color:#fff;border-radius:3px;padding:1px 5px;font-size:10px}
.group{font:600 10px ui-monospace,monospace;letter-spacing:.12em;text-transform:uppercase;color:#787F88;margin:10px 0 6px}
.item{display:block;width:100%;text-align:left;background:none;border:0;border-radius:5px;padding:6px 9px;font:inherit;color:#4A5058;cursor:pointer}
.item:hover{background:#E7E6E1}
.item.sel{background:#C4451B;color:#fff;font-weight:500}
.hint{margin-top:auto;font-size:11px;line-height:1.5;color:#787F88;border-top:1px solid #E2E1DB;padding-top:10px}
.main{display:flex;flex-direction:column;min-width:0}
.toolbar{display:flex;align-items:center;gap:10px;padding:10px 18px;border-bottom:1px solid #E2E1DB;background:#fff}
.crumb{font-weight:600} .spacer{flex:1}
.id{font:11px ui-monospace,monospace;color:#787F88}
.canvas{flex:1;display:grid;place-items:center;padding:40px;background:#fff;min-height:180px}
.panels{display:grid;grid-template-columns:1fr 260px;border-top:1px solid #E2E1DB;background:#F2F2EF;max-height:46vh;overflow:auto}
.panel{padding:14px 18px;border-right:1px solid #E2E1DB}
.panel h3{font:600 10px ui-monospace,monospace;letter-spacing:.12em;text-transform:uppercase;color:#787F88;margin:0 0 10px}
table.controls{width:100%;border-collapse:collapse}
table.controls th{text-align:left;font-weight:400;padding:7px 10px 7px 0;vertical-align:top;border-bottom:1px solid #E2E1DB;width:44%}
table.controls td{padding:7px 0;vertical-align:top;border-bottom:1px solid #E2E1DB}
.nm{font:600 13px ui-monospace,monospace}
.ty{font:11px ui-monospace,monospace;color:#787F88}
.doc{font-size:11.5px;color:#4A5058;margin-top:3px;max-width:34ch}
.muted{color:#787F88;font-size:12px}
.row{display:flex;align-items:center;gap:8px}
input[type=text],input[type=number],select{width:100%;padding:5px 8px;border:1px solid #CFCEC7;border-radius:5px;font:inherit;background:#fff}
input[type=range]{flex:1}
.reset{margin-top:12px;background:#fff;border:1px solid #CFCEC7;border-radius:5px;padding:5px 11px;font:inherit;cursor:pointer}
.actions{list-style:none;margin:0;padding:0;font:12px ui-monospace,monospace}
.actions li{padding:3px 0;border-bottom:1px solid #E2E1DB;color:#4A5058}
"#;
