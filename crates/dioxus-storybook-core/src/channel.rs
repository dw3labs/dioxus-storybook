//! The manager ↔ preview event bus.
//!
//! In Storybook the manager and the preview are two documents talking over
//! `postMessage`, and every feature — controls, actions, a11y — is an addon
//! plugged into that bus rather than a privileged core capability.
//!
//! M1 runs both halves in one Dioxus app, so [`InProcessChannel`] is a direct
//! function call. The [`Channel`] trait exists anyway, from the first commit,
//! because M3 swaps in a `postMessage` implementation and the point is that no
//! addon has to change when it does.
//!
//! # Threading
//!
//! Web-first: listeners are `Rc`-held and this type is single-threaded by
//! design. It is not `Send`/`Sync` and is not meant to be.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::ArgMap;

/// A message on the bus.
///
/// Marked `#[non_exhaustive]`: later milestones add globals, viewport, a11y and
/// interaction events, so downstream `match`es need a wildcard arm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// The manager selected a story. The preview should render it.
    SetCurrentStory {
        /// The story's [`StoryDef::id`](crate::StoryDef::id).
        id: String,
    },
    /// The manager changed the args for the current story.
    UpdateArgs {
        /// The story the args belong to.
        id: String,
        /// The full arg set, not a delta.
        args: ArgMap,
    },
    /// The manager asked to drop overrides and fall back to story defaults.
    ResetArgs {
        /// The story the reset applies to.
        id: String,
    },
    /// The preview finished rendering a story.
    StoryRendered {
        /// The story that rendered.
        id: String,
    },
    /// The preview was asked for a story the registry does not contain —
    /// usually a stale URL.
    StoryMissing {
        /// The requested id.
        id: String,
    },
    /// A component called an event handler wired to the actions panel.
    ActionLogged {
        /// The prop name, e.g. `onclick`.
        name: String,
        /// A rendered description of the payload.
        payload: String,
    },
}

/// A subscribed callback.
pub type Listener = Rc<dyn for<'a> Fn(&'a Event)>;

/// One entry in a channel's listener list: its id and the callback.
type Entry = (u64, Listener);

/// A transport between the manager and the preview.
///
/// Implementations must tolerate a listener emitting during dispatch; see
/// [`InProcessChannel`] for the reference behaviour.
pub trait Channel {
    /// Broadcast `event` to every current listener.
    fn emit(&self, event: Event);

    /// Register `listener`. It stays subscribed until the returned
    /// [`Subscription`] is dropped.
    fn subscribe(&self, listener: Listener) -> Subscription;
}

/// Keeps a subscription alive. Dropping it unsubscribes.
#[derive(Debug)]
pub struct Subscription {
    inner: Weak<Inner>,
    id: u64,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        // Spelled out rather than inferred: rust-analyzer cannot type this
        // through `Weak::upgrade` and reports a false `type annotations needed`.
        let inner: Rc<Inner> = match self.inner.upgrade() {
            Some(inner) => inner,
            None => return,
        };
        let mut listeners = inner.listeners.borrow_mut();
        listeners.retain(|entry: &Entry| entry.0 != self.id);
    }
}

#[derive(Default)]
struct Inner {
    next_id: Cell<u64>,
    listeners: RefCell<Vec<Entry>>,
}

impl core::fmt::Debug for Inner {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Inner")
            .field("listeners", &self.listeners.borrow().len())
            .finish()
    }
}

/// The M1 transport: both halves live in one app, so an emit is a direct call.
///
/// Cheap to clone — clones share one listener list.
#[derive(Clone, Default, Debug)]
pub struct InProcessChannel {
    inner: Rc<Inner>,
}

impl InProcessChannel {
    /// A channel with no listeners.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many listeners are currently subscribed. Useful in tests.
    pub fn listener_count(&self) -> usize {
        self.inner.listeners.borrow().len()
    }
}

impl Channel for InProcessChannel {
    fn emit(&self, event: Event) {
        // Snapshot before dispatch: a listener is allowed to emit, subscribe or
        // unsubscribe while being called, and must not deadlock the RefCell.
        let listeners: Vec<Listener> = self
            .inner
            .listeners
            .borrow()
            .iter()
            .map(|(_, f)| Rc::clone(f))
            .collect();
        for listener in listeners {
            listener(&event);
        }
    }

    fn subscribe(&self, listener: Listener) -> Subscription {
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id + 1);
        self.inner.listeners.borrow_mut().push((id, listener));
        Subscription {
            inner: Rc::downgrade(&self.inner),
            id,
        }
    }
}
