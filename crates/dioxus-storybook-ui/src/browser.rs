//! The thin layer that touches the browser: reading the query string, writing
//! it back, and moving focus.
//!
//! Everything here is a no-op off `wasm32`, so the rest of the crate compiles
//! and unit-tests on the host.

use dioxus_storybook_core::UrlState;

/// Read the current story and args out of `window.location.search`.
///
/// This is how the dev loop survives a rebuild: `dx serve` cannot hot-patch on
/// wasm, so every source edit reloads the page and destroys every signal. The
/// URL is the only state that comes back.
#[cfg(target_arch = "wasm32")]
pub fn read_url_state() -> UrlState {
    let Some(window) = web_sys::window() else {
        return UrlState::default();
    };
    match window.location().search() {
        Ok(search) => UrlState::parse(&search),
        Err(_) => UrlState::default(),
    }
}

/// Off wasm there is no location to read.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_url_state() -> UrlState {
    UrlState::default()
}

/// Write the current story and args into the address bar.
///
/// Uses `replaceState`, not `pushState`: browsing stories should not fill the
/// back button with every arg keystroke.
#[cfg(target_arch = "wasm32")]
pub fn write_url_state(state: &UrlState) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(history) = window.history() else {
        return;
    };
    let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&state.to_query()));
}

/// Off wasm there is no address bar to write to.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_url_state(_state: &UrlState) {}

/// Move keyboard focus to the element with the given id.
#[cfg(target_arch = "wasm32")]
pub fn focus(element_id: &str) {
    use wasm_bindgen::JsCast;
    let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(element_id))
    else {
        return;
    };
    if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
        let _ = el.focus();
    }
}

/// Off wasm there is nothing to focus.
#[cfg(not(target_arch = "wasm32"))]
pub fn focus(_element_id: &str) {}
