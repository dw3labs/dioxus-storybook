//! A pair of documents, on the host.
//!
//! From M3 the manager and the preview are two *documents*: two DOM trees, two
//! Dioxus runtimes, no shared memory, and a string on a wire between them. This
//! harness reproduces that arrangement without a browser — two independent
//! `VirtualDom`s joined by a channel pair that carries nothing but the encoded
//! bytes.
//!
//! Two properties of the real transport are reproduced on purpose:
//!
//! - **A document does not hear itself.** `emit` reaches the *peer's* listeners
//!   only, unlike [`InProcessChannel`], which broadcasts to everyone. If either
//!   half ever starts relying on its own echo, these tests fail and the browser
//!   does not.
//! - **Delivery is asynchronous.** `postMessage` queues a task; it does not call
//!   into the other document mid-render. So an `emit` here enqueues, and
//!   [`Documents::settle`] delivers between passes. Doing it synchronously would
//!   also write one dom's signals from inside the other's runtime, which is not
//!   a thing the browser ever does.
//!
//! [`InProcessChannel`]: dioxus_storybook_core::InProcessChannel

#![allow(dead_code)] // each test file uses a different part of this

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

use dioxus::prelude::*;
use dioxus_storybook_core::wire::{decode, encode};
use dioxus_storybook_core::{
    Channel, ChannelHandle, Event, Listener, Project, Registry, StoryDef, Subscription, ViewMode,
};
use dioxus_storybook_ui::{
    StorybookManager, StorybookManagerProps, StorybookPreview, StorybookPreviewProps,
};

type Entry = (u64, Listener);

/// The wire between the two documents. Holds encoded strings, not events.
#[derive(Default)]
pub struct Bus {
    queue: RefCell<VecDeque<(ViewMode, String)>>,
    manager: RefCell<Vec<Entry>>,
    preview: RefCell<Vec<Entry>>,
    next_id: RefCell<u64>,
    /// Every message that has crossed, in order, for assertions.
    pub traffic: RefCell<Vec<(ViewMode, Event)>>,
}

impl Bus {
    fn listeners(&self, mode: ViewMode) -> &RefCell<Vec<Entry>> {
        match mode {
            ViewMode::Preview => &self.preview,
            _ => &self.manager,
        }
    }

    fn pop(&self) -> Option<(ViewMode, String)> {
        self.queue.borrow_mut().pop_front()
    }

    fn snapshot(&self, mode: ViewMode) -> Vec<Listener> {
        self.listeners(mode)
            .borrow()
            .iter()
            .map(|(_, f)| Rc::clone(f))
            .collect()
    }

    /// Put a message on the wire as if `from` had sent it.
    ///
    /// The shell's own controls are DOM events, and a headless `VirtualDom` has
    /// no way to deliver one — so a test that wants to see the *other* document
    /// react posts the message the widget would have caused. It goes through the
    /// codec like any other, so a message that could not survive the wire cannot
    /// be injected either.
    pub fn inject(&self, from: ViewMode, event: Event) {
        self.queue.borrow_mut().push_back((from, encode(from, &event)));
    }

    /// Every event sent by `mode`, in order.
    pub fn sent_by(&self, mode: ViewMode) -> Vec<Event> {
        self.traffic
            .borrow()
            .iter()
            .filter(|(from, _)| *from == mode)
            .map(|(_, event)| event.clone())
            .collect()
    }
}

/// One document's end of the wire.
struct DocChannel {
    mode: ViewMode,
    bus: Rc<Bus>,
}

impl Channel for DocChannel {
    fn emit(&self, event: Event) {
        // Encoded at send time, exactly as `postMessage` would: a message that
        // cannot survive the codec must fail here, not silently pass because
        // the test kept the Rust value around.
        let raw = encode(self.mode, &event);
        self.bus.queue.borrow_mut().push_back((self.mode, raw));
    }

    fn subscribe(&self, listener: Listener) -> Subscription {
        let id = {
            let mut next = self.bus.next_id.borrow_mut();
            *next += 1;
            *next
        };
        self.bus.listeners(self.mode).borrow_mut().push((id, listener));
        let weak: Weak<Bus> = Rc::downgrade(&self.bus);
        let mode = self.mode;
        Subscription::new(move || {
            let bus: Rc<Bus> = match weak.upgrade() {
                Some(bus) => bus,
                None => return,
            };
            bus.listeners(mode)
                .borrow_mut()
                .retain(|entry: &Entry| entry.0 != id);
        })
    }
}

/// A manager and a preview, wired together.
pub struct Documents {
    pub manager: VirtualDom,
    pub preview: VirtualDom,
    pub bus: Rc<Bus>,
}

impl Documents {
    /// Mount both halves over the same registry and let them introduce
    /// themselves.
    pub fn new(stories: &'static [&'static StoryDef]) -> Self {
        Self::with_project(stories, Project::new())
    }

    /// The same pair, with project-level decorators and parameters in force.
    pub fn with_project(stories: &'static [&'static StoryDef], project: Project) -> Self {
        let registry = Registry::new(stories);
        let bus = Rc::new(Bus::default());

        let manager = VirtualDom::new_with_props(
            StorybookManager,
            StorybookManagerProps {
                registry,
                channel: ChannelHandle::new(DocChannel {
                    mode: ViewMode::Manager,
                    bus: Rc::clone(&bus),
                }),
            },
        );
        let preview = VirtualDom::new_with_props(
            StorybookPreview,
            StorybookPreviewProps {
                registry,
                project,
                channel: ChannelHandle::new(DocChannel {
                    mode: ViewMode::Preview,
                    bus: Rc::clone(&bus),
                }),
            },
        );

        let mut docs = Self { manager, preview, bus };
        docs.manager.rebuild_in_place();
        docs.preview.rebuild_in_place();
        docs.settle();
        docs
    }

    /// Render, run effects, deliver, repeat — until nothing is left to say.
    ///
    /// A browser reaches this state on its own over a few frames. A bare
    /// `VirtualDom` only advances when asked, and in particular **does not run
    /// effects** unless `process_events` is called with no dirty scopes left —
    /// which is why the shell's publishing effect appears to do nothing in a
    /// test that only calls `rebuild_in_place`.
    pub fn settle(&mut self) {
        for _ in 0..12 {
            self.pump();
            if self.bus.queue.borrow().is_empty() {
                return;
            }
            self.deliver();
        }
        panic!("the two documents never went quiet");
    }

    fn pump(&mut self) {
        for _ in 0..4 {
            let manager = self.manager.render_immediate_to_vec();
            self.manager.process_events();
            let preview = self.preview.render_immediate_to_vec();
            self.preview.process_events();
            if manager.edits.is_empty() && preview.edits.is_empty() {
                return;
            }
        }
    }

    /// Hand every queued message to the other document's listeners.
    ///
    /// Each delivery runs inside the *recipient's* runtime, because that is
    /// where it happens in a browser: a document has exactly one Dioxus runtime
    /// and its `onmessage` handler writes that document's signals.
    fn deliver(&mut self) {
        for _ in 0..64 {
            let Some((from, raw)) = self.bus.pop() else {
                return;
            };
            let to = from.peer();
            let Some(event) = decode(to, &raw) else {
                panic!("a message did not survive the wire: {raw}");
            };
            self.bus.traffic.borrow_mut().push((from, event.clone()));
            let listeners = self.bus.snapshot(to);
            let dom = match to {
                ViewMode::Preview => &self.preview,
                _ => &self.manager,
            };
            dom.in_runtime(|| {
                for listener in &listeners {
                    listener(&event);
                }
            });
        }
        panic!("the two documents are talking in circles");
    }

    pub fn manager_html(&self) -> String {
        dioxus_ssr::render(&self.manager)
    }

    pub fn preview_html(&self) -> String {
        dioxus_ssr::render(&self.preview)
    }
}

/// A preview with nobody on the other end, rendered once.
///
/// The framed document resolves its own landing story from its own URL rather
/// than waiting to be told — the same rule the manager uses, so the two answers
/// cannot drift. This is what proves it: no manager exists, no message is ever
/// delivered, and the story is on screen anyway.
pub fn unpaired_preview_html(stories: &'static [&'static StoryDef]) -> String {
    let bus = Rc::new(Bus::default());
    let mut dom = VirtualDom::new_with_props(
        StorybookPreview,
        StorybookPreviewProps {
            registry: Registry::new(stories),
            project: Project::new(),
            channel: ChannelHandle::new(DocChannel { mode: ViewMode::Preview, bus }),
        },
    );
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}
