//! S4 — the M3 isolation spike.
//!
//! QUESTION: M3 wants the preview isolated in an iframe. The plan said "two
//! wasm bundles", but `dx` builds one binary per app. This spike asks whether
//! ONE bundle, loaded twice — once as the page, once inside an iframe pointed
//! at the same URL with `?role=preview` — is enough.
//!
//! What has to be true:
//!   1. The iframe, given a query-string URL on the same origin, loads the same
//!      app under `dx serve --platform web` (no second HTML file, no dx config).
//!   2. parent -> child `postMessage` arrives.
//!   3. child -> parent `postMessage` arrives.
//!   4. A message handler (a plain JS callback, no Dioxus scope) can WRITE a
//!      signal and have the owning component re-render. This is the one that
//!      could sink the design.
//!
//! Run: cd spikes/s4-iframe && dx serve --platform web
//! Expect: the page shows "child says: pong 1", "pong 2", ... as you click, and
//! the framed half shows the pings it received. Both halves count up.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

/// One root, two roles, chosen by the query string — exactly what the real
/// `Storybook` component would do.
#[component]
fn App() -> Element {
    let is_preview = use_hook(|| platform::query().contains("role=preview"));
    if is_preview {
        rsx! { Child {} }
    } else {
        rsx! { Parent {} }
    }
}

/// The manager half: owns the iframe, sends pings, displays what comes back.
#[component]
fn Parent() -> Element {
    // Written from a JS message callback, read here. Rooted at ROOT for the
    // same reason the real preview roots its signals: the writer is not a
    // descendant scope.
    let from_child = use_signal(|| Vec::<String>::new());
    let ready = use_signal(|| false);
    let mut seq = use_signal(|| 0u32);

    use_hook(|| {
        platform::on_message(move |data: String| {
            let mut from_child = from_child;
            let mut ready = ready;
            if data == "ready" {
                ready.set(true);
            }
            from_child.write().push(data);
        })
    });

    rsx! {
        div { style: "font:14px system-ui;padding:16px",
            h2 { "S4 — parent" }
            p { "child ready: {ready}" }
            button {
                onclick: move |_| {
                    let n = seq() + 1;
                    seq.set(n);
                    platform::post_to_iframe("dxsb-frame", &format!("ping {n}"));
                },
                "ping the frame"
            }
            ul {
                for (i, msg) in from_child.read().iter().enumerate() {
                    li { key: "{i}", "child says: {msg}" }
                }
            }
            iframe {
                id: "dxsb-frame",
                src: "?role=preview",
                style: "width:100%;height:220px;border:1px solid #999",
            }
        }
    }
}

/// The preview half: announces itself, echoes every ping back to the parent.
#[component]
fn Child() -> Element {
    let seen = use_signal(|| Vec::<String>::new());

    use_hook(|| {
        platform::on_message(move |data: String| {
            let mut seen = seen;
            seen.write().push(data.clone());
            platform::post_to_parent(&format!("pong {}", data.trim_start_matches("ping ")));
        });
        // Announce readiness AFTER subscribing, so a parent that replies
        // immediately cannot beat us to the listener.
        platform::post_to_parent("ready");
    });

    rsx! {
        div { style: "font:14px system-ui;padding:16px;background:#f4f4f8",
            h3 { "S4 — child (in the frame)" }
            ul {
                for (i, msg) in seen.read().iter().enumerate() {
                    li { key: "{i}", "parent said: {msg}" }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::prelude::*;

    pub fn query() -> String {
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default()
    }

    /// Subscribe to `window.onmessage`. The closure is leaked: it lives as long
    /// as the document, which for a spike is exactly right.
    pub fn on_message(f: impl Fn(String) + 'static) {
        let Some(window) = web_sys::window() else { return };
        let cb = Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            if let Some(s) = e.data().as_string() {
                f(s);
            }
        });
        let _ = window
            .add_event_listener_with_callback("message", cb.as_ref().unchecked_ref());
        cb.forget();
    }

    pub fn post_to_parent(msg: &str) {
        let Some(window) = web_sys::window() else { return };
        let Ok(parent) = window.parent() else { return };
        let Some(parent) = parent else { return };
        let _ = parent.post_message(&JsValue::from_str(msg), "*");
    }

    pub fn post_to_iframe(id: &str, msg: &str) {
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
        let Some(el) = doc.get_element_by_id(id) else { return };
        let Ok(frame) = el.dyn_into::<web_sys::HtmlIFrameElement>() else { return };
        let Some(win) = frame.content_window() else { return };
        let _ = win.post_message(&JsValue::from_str(msg), "*");
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    pub fn query() -> String { String::new() }
    pub fn on_message(_f: impl Fn(String) + 'static) {}
    pub fn post_to_parent(_msg: &str) {}
    pub fn post_to_iframe(_id: &str, _msg: &str) {}
}
