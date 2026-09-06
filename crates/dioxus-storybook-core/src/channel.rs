//! The manager ↔ preview event bus.
//!
//! In Storybook the manager and the preview are two documents talking over
//! `postMessage`, and every feature — controls, actions, a11y — is an addon
//! plugged into that bus rather than a privileged core capability.
//!
//! M1 and M2 ran both halves in one Dioxus app, where [`InProcessChannel`] is a
//! direct function call. The [`Channel`] trait existed anyway, from the first
//! commit, so that M3 could put the preview in an iframe by writing one new
//! implementation — which is what happened: `dioxus-storybook-ui` supplies a
//! `postMessage` transport and not a single addon changed. The wire form of an
//! [`Event`] lives in [`crate::wire`].
//!
//! # The one behavioural difference between transports
//!
//! [`InProcessChannel::emit`] reaches **every** listener, the emitter's own
//! included. `postMessage` reaches only the other document. Nothing in this
//! crate or in the shell may therefore depend on hearing its own messages —
//! `dioxus-storybook-ui`'s `tests/two_documents.rs` holds that line.
//!
//! # Threading
//!
//! Web-first: listeners are `Rc`-held and this type is single-threaded by
//! design. It is not `Send`/`Sync` and is not meant to be.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::ArgMap;

/// A message on the bus.
///
/// Marked `#[non_exhaustive]`: later milestones add globals, viewport, a11y and
/// interaction events, so downstream `match`es need a wildcard arm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// The preview document has mounted and subscribed.
    ///
    /// Only meaningful once the two halves are separate documents: the manager
    /// renders its iframe and starts emitting immediately, but the preview does
    /// not exist yet, so those first messages would fall on the floor. The
    /// manager holds them until this arrives. It is broadcast, not addressed —
    /// a preview that reloads on its own (a `dx serve` rebuild, say) sends it
    /// again and gets the current state back.
    PreviewReady,
    /// The manager selected a story. The preview should render it.
    SetCurrentStory {
        /// The story's [`StoryDef::id`](crate::StoryDef::id).
        id: String,
    },
    /// The manager changed the args for the current story.
    ///
    /// Authoritative and complete: the receiver replaces its arg set with this
    /// one rather than merging. Compare [`Event::RequestArgsUpdate`], which
    /// travels the other way and *is* a delta.
    UpdateArgs {
        /// The story the args belong to.
        id: String,
        /// The full arg set, not a delta.
        args: ArgMap,
    },
    /// The preview asks the manager to merge some args into the current set.
    ///
    /// Sent when a story writes its own args back through
    /// [`ArgsHandle`](crate::ArgsHandle) — a controlled input, say. The manager
    /// merges and re-broadcasts as [`Event::UpdateArgs`], which is what stops
    /// the two sides from keeping divergent copies.
    RequestArgsUpdate {
        /// The story the args belong to.
        id: String,
        /// Only the args that changed.
        args: ArgMap,
    },
    /// The preview evaluated a story's typed props and is publishing the
    /// defaults they imply.
    ///
    /// The controls panel needs a story's default values to show what a control
    /// is currently sitting at, but the manager must not compute them itself:
    /// evaluating props requires a Dioxus scope and, from M3, the story
    /// functions live in the other bundle entirely. So the preview computes
    /// them once per story and sends them across.
    StoryPrepared {
        /// The story that was prepared.
        id: String,
        /// The story's own arg values, before any URL or panel overlay.
        initial_args: ArgMap,
    },
    /// The manager changed the toolbar globals.
    ///
    /// Authoritative and complete, like [`Event::UpdateArgs`] — the receiver
    /// replaces its set rather than merging. Only the *selected* values travel;
    /// each side fills in the declared defaults itself, because the
    /// declarations are `&'static` and both halves share the bundle.
    SetGlobals {
        /// Every global the toolbar has been moved off its default.
        globals: ArgMap,
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
    /// The preview document died.
    ///
    /// A story is arbitrary user code and it can panic. Before M3 that was
    /// loud — the whole app went down and the console said why. Now the shell
    /// survives its own frame, so a panic would leave a workbench that looks
    /// fine next to a canvas that has silently stopped. The preview reports its
    /// own death from a panic hook, on the way out.
    ///
    /// It really is the way out: wasm builds with `panic = "abort"` (a hard
    /// requirement for `dx`, see the project log), so the hook runs and then the
    /// module is gone. Nothing after this message will arrive, and only
    /// reloading the frame brings the preview back.
    PreviewPanicked {
        /// What the panic said, with its location if the hook had one.
        message: String,
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
///
/// It carries the undo rather than a reference to the channel that made it, so
/// a [`Channel`] implemented outside this crate — the `postMessage` transport
/// in `dioxus-storybook-ui`, and any addon transport later — can return one.
pub struct Subscription {
    on_drop: Option<Box<dyn FnOnce()>>,
}

impl Subscription {
    /// A subscription that runs `unsubscribe` when it is dropped.
    pub fn new(unsubscribe: impl FnOnce() + 'static) -> Self {
        Self {
            on_drop: Some(Box::new(unsubscribe)),
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(undo) = self.on_drop.take() {
            undo();
        }
    }
}

impl core::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Subscription")
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
        // Weak: a live subscription must not keep the channel alive.
        let weak = Rc::downgrade(&self.inner);
        Subscription::new(move || {
            // Spelled out rather than inferred: rust-analyzer cannot type this
            // through `Weak::upgrade` and reports a false
            // `type annotations needed`.
            let inner: Rc<Inner> = match weak.upgrade() {
                Some(inner) => inner,
                None => return,
            };
            inner
                .listeners
                .borrow_mut()
                .retain(|entry: &Entry| entry.0 != id);
        })
    }
}

/// A [`Channel`] whose implementation is chosen at run time.
///
/// The transport is not known statically any more: a document is the manager or
/// the preview depending on its own URL, and off the browser it is neither. So
/// the bus travels through the component tree as this handle rather than as a
/// concrete type, and `use_context::<ChannelHandle>()` means the same thing in
/// every arrangement.
///
/// Cheap to clone — clones share one implementation.
#[derive(Clone)]
pub struct ChannelHandle(Rc<dyn Channel>);

impl ChannelHandle {
    /// Wrap a concrete transport.
    pub fn new(channel: impl Channel + 'static) -> Self {
        Self(Rc::new(channel))
    }
}

impl Channel for ChannelHandle {
    fn emit(&self, event: Event) {
        self.0.emit(event);
    }

    fn subscribe(&self, listener: Listener) -> Subscription {
        self.0.subscribe(listener)
    }
}

impl core::fmt::Debug for ChannelHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ChannelHandle(..)")
    }
}

/// Two handles are equal when they wrap the same implementation.
///
/// Needed because a handle can travel as a Dioxus prop, and props are compared
/// to decide whether to re-render.
impl PartialEq for ChannelHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
