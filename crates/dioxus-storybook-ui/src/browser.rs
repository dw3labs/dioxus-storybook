//! The thin layer that touches the browser: reading the query string, writing
//! it back, and moving focus.
//!
//! Everything here is a no-op off `wasm32`, so the rest of the crate compiles
//! and unit-tests on the host.

use dioxus_storybook_core::wire::VIEW_MODE_PARAM;
use dioxus_storybook_core::{ChannelHandle, UrlState, ViewMode};

#[cfg(not(target_arch = "wasm32"))]
use dioxus_storybook_core::InProcessChannel;

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

// ---------------------------------------------------------------- M3: roles

/// Which half of the storybook this document is: read from its own URL.
///
/// One wasm bundle serves both roles. The manager points an iframe at the same
/// URL with `?viewMode=preview`, and the copy that loads there sees the
/// parameter and renders the canvas alone. See [`crate::Storybook`].
#[cfg(target_arch = "wasm32")]
pub fn view_mode() -> ViewMode {
    let Some(window) = web_sys::window() else {
        return ViewMode::Manager;
    };
    let Ok(search) = window.location().search() else {
        return ViewMode::Manager;
    };
    ViewMode::from_query(&search)
}

/// Off wasm there is one document and it is the manager.
#[cfg(not(target_arch = "wasm32"))]
pub fn view_mode() -> ViewMode {
    ViewMode::Manager
}

/// The `src` for the preview iframe: the same document, in the other role.
///
/// Carries the story and args so the framed copy paints the right thing on its
/// first render, before any message has crossed — the same reason the manager
/// reads them from its own URL. It is computed **once**, at mount: putting live
/// state in `src` would reload the iframe on every keystroke.
pub fn preview_src(state: &UrlState) -> String {
    let query = state.to_query();
    let sep = if query == "?" { "" } else { "&" };
    format!("{query}{sep}{VIEW_MODE_PARAM}={}", ViewMode::Preview.as_str())
}

/// The bus for this document.
///
/// On the browser this is `postMessage` to the other half. Off it — host tests,
/// and anything that renders the shell without a DOM — there is no other half,
/// so both ends of the conversation are the one in-process channel.
#[cfg(target_arch = "wasm32")]
pub fn channel_for(mode: ViewMode, frame_id: &str) -> ChannelHandle {
    ChannelHandle::new(post_message::PostMessageChannel::new(mode, frame_id))
}

/// Off wasm, the in-process channel — the M1/M2 arrangement.
#[cfg(not(target_arch = "wasm32"))]
pub fn channel_for(_mode: ViewMode, _frame_id: &str) -> ChannelHandle {
    ChannelHandle::new(InProcessChannel::new())
}

/// Make the preview document report its own death.
///
/// A story is arbitrary user code. Before M3 a panic in one took the whole page
/// with it, which at least *looked* like something had gone wrong; now the shell
/// lives in a different document and would carry on looking healthy next to a
/// frozen frame. So the preview installs a panic hook that posts
/// [`Event::PreviewPanicked`] to the manager on the way out.
///
/// Three things make this a hook rather than a `catch_unwind`:
///
/// - wasm builds here are `panic = "abort"`. Nothing can be caught; the most
///   that can be done is to say so before the module stops.
/// - `set_hook` demands `Send + Sync`, and the channel is `Rc`-based. The
///   closure captures nothing but the previous hook and looks `window` up
///   afresh, so it satisfies the bound without the transport being thread-safe.
/// - The previous hook is chained, not replaced. Dioxus installs one that
///   prints to the console, and losing that would trade one silence for another.
#[cfg(target_arch = "wasm32")]
pub fn report_panics_to_manager() {
    use dioxus_storybook_core::Event;
    use dioxus_storybook_core::wire::encode;
    use wasm_bindgen::prelude::*;

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Say it before doing anything that could itself fail.
        let event = Event::PreviewPanicked { message: info.to_string() };
        if let Some(parent) = web_sys::window().and_then(|w| w.parent().ok().flatten()) {
            let raw = encode(ViewMode::Preview, &event);
            let _ = parent.post_message(&JsValue::from_str(&raw), "*");
        }
        previous(info);
    }));
}

/// Off wasm there is no manager to tell, and installing a process-wide panic
/// hook from a library would be a rude thing to do to a test runner.
#[cfg(not(target_arch = "wasm32"))]
pub fn report_panics_to_manager() {}

#[cfg(target_arch = "wasm32")]
mod post_message {
    //! The real M3 transport.
    //!
    //! An `emit` here does **not** reach this document's own listeners, only the
    //! peer's. That is a real difference from [`InProcessChannel`], which
    //! broadcasts to everyone including the emitter — and it is safe only
    //! because neither half acts on its own messages. `tests/two_documents.rs`
    //! pins that property.

    use std::cell::{Cell, RefCell};
    use std::rc::{Rc, Weak};

    use dioxus_storybook_core::wire::{decode, encode};
    use dioxus_storybook_core::{Channel, Event, Listener, Subscription, ViewMode};
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::prelude::*;

    type Entry = (u64, Listener);

    pub struct PostMessageChannel {
        inner: Rc<Inner>,
    }

    struct Inner {
        mode: ViewMode,
        /// The id of the iframe element to post into. Unused by the preview,
        /// whose peer is `window.parent`.
        frame_id: String,
        listeners: RefCell<Vec<Entry>>,
        next_id: Cell<u64>,
        /// Kept so the listener can be removed when the channel is dropped.
        on_message: RefCell<Option<Closure<dyn Fn(web_sys::MessageEvent)>>>,
    }

    impl PostMessageChannel {
        pub fn new(mode: ViewMode, frame_id: &str) -> Self {
            let inner = Rc::new(Inner {
                mode,
                frame_id: frame_id.to_string(),
                listeners: RefCell::new(Vec::new()),
                next_id: Cell::new(0),
                on_message: RefCell::new(None),
            });

            if let Some(window) = web_sys::window() {
                // Weak, so the closure the browser holds does not keep the
                // channel alive after the component that owns it is gone.
                let weak: Weak<Inner> = Rc::downgrade(&inner);
                let cb = Closure::<dyn Fn(web_sys::MessageEvent)>::new(
                    move |event: web_sys::MessageEvent| {
                        let Some(inner): Option<Rc<Inner>> = weak.upgrade() else {
                            return;
                        };
                        let Some(raw) = event.data().as_string() else {
                            return;
                        };
                        // Not ours, or our own voice coming back. Either way,
                        // not an event.
                        let Some(event) = decode(inner.mode, &raw) else {
                            return;
                        };
                        inner.dispatch(&event);
                    },
                );
                let _ = window
                    .add_event_listener_with_callback("message", cb.as_ref().unchecked_ref());
                *inner.on_message.borrow_mut() = Some(cb);
            }

            Self { inner }
        }
    }

    impl Inner {
        /// Deliver to local listeners. Snapshot first: a listener may subscribe
        /// or unsubscribe while being called.
        fn dispatch(&self, event: &Event) {
            let listeners: Vec<Listener> = self
                .listeners
                .borrow()
                .iter()
                .map(|(_, f)| Rc::clone(f))
                .collect();
            for listener in listeners {
                listener(event);
            }
        }

        /// The window on the other end, resolved per send.
        ///
        /// Per send rather than once, because the manager's iframe may not exist
        /// yet when the channel is built, and it is replaced outright when the
        /// preview reloads.
        fn peer_window(&self) -> Option<web_sys::Window> {
            match self.mode {
                ViewMode::Preview => web_sys::window()?.parent().ok().flatten(),
                _ => {
                    let doc = web_sys::window()?.document()?;
                    let frame = doc
                        .get_element_by_id(&self.frame_id)?
                        .dyn_into::<web_sys::HtmlIFrameElement>()
                        .ok()?;
                    frame.content_window()
                }
            }
        }
    }

    impl Drop for Inner {
        fn drop(&mut self) {
            let (Some(window), Some(cb)) = (web_sys::window(), self.on_message.take()) else {
                return;
            };
            let _ = window
                .remove_event_listener_with_callback("message", cb.as_ref().unchecked_ref());
        }
    }

    impl Channel for PostMessageChannel {
        fn emit(&self, event: Event) {
            // A message sent before the peer document exists is dropped, not
            // queued. Every manager -> preview message is a state sync rather
            // than a command, and the preview asks for the current state with
            // `PreviewReady` the moment it mounts — so a queue would only make
            // the same messages arrive twice.
            let Some(peer) = self.inner.peer_window() else {
                return;
            };
            let raw = encode(self.inner.mode, &event);
            // `*` as the target origin: the two documents are the same bundle
            // on the same origin, and a stricter value would have to be
            // reconstructed from `location`, which buys nothing here. Every
            // reader validates the envelope instead.
            let _ = peer.post_message(&JsValue::from_str(&raw), "*");
        }

        fn subscribe(&self, listener: Listener) -> Subscription {
            let id = self.inner.next_id.get();
            self.inner.next_id.set(id + 1);
            self.inner.listeners.borrow_mut().push((id, listener));
            let weak: Weak<Inner> = Rc::downgrade(&self.inner);
            Subscription::new(move || {
                let inner: Rc<Inner> = match weak.upgrade() {
                    Some(inner) => inner,
                    None => return,
                };
                inner.listeners.borrow_mut().retain(|entry: &Entry| entry.0 != id);
            })
        }
    }
}
