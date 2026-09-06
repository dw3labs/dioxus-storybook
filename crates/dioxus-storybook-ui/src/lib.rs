//! The dioxus-storybook manager shell: sidebar, search, keyboard navigation,
//! the preview harness, and URL state.
//!
//! # Two halves, one bundle — for now
//!
//! Storybook proper runs the manager and the preview as two documents talking
//! over `postMessage`. M1 runs both in a single Dioxus app, but the manager
//! never calls the preview directly: it emits [`Event`]s on a [`Channel`] and
//! the preview subscribes. That indirection looks like ceremony today and is
//! the whole point tomorrow — M3 swaps an iframe transport underneath without
//! any component here changing.
//!
//! [`Channel`]: dioxus_storybook_core::Channel
//! [`Event`]: dioxus_storybook_core::Event

#![deny(missing_docs)]

mod browser;
mod panels;
mod style;

use std::collections::BTreeSet;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_storybook_core::{
    ActionSink, ArgMap, ArgValue, ArgsHandle, Channel, Event, InProcessChannel, Registry, Row,
    RowKind, StoryDef, UrlState, flatten,
};

use panels::{ACTION_LOG_CAP, ActionEntry, AddonPanel, PanelTab};

pub use style::MANAGER_CSS;

/// The id of the search input, so `/` can focus it.
const SEARCH_ID: &str = "dxsb-search";

/// Which story to show on load: the one the URL names if it still exists,
/// otherwise the first in the index.
///
/// The manager and the preview each resolve this independently — in Storybook
/// the preview is an iframe with its own `?id=` URL, and keeping that true here
/// is what makes the M3 split a transport change rather than a rewrite. They
/// share this one function so the two answers cannot drift apart.
fn landing_story(registry: Registry, requested: Option<&str>) -> Option<String> {
    requested
        .filter(|id| registry.get(id).is_some())
        .map(str::to_string)
        .or_else(|| registry.first().map(StoryDef::id))
}

/// The storybook manager.
///
/// Mount it as your whole app:
///
/// ```ignore
/// fn main() {
///     dioxus::launch(|| rsx! { Storybook { registry: stories::registry() } });
/// }
/// ```
#[component]
pub fn Storybook(registry: Registry) -> Element {
    // The bus. Provided as context so the preview — and, from M2, every addon
    // panel — talks to the manager without either holding the other's signals.
    let channel: InProcessChannel = use_context_provider(InProcessChannel::new);

    // The URL is the only state that survives a rebuild, so it is the source of
    // truth at startup, not a mirror written afterwards.
    let initial = use_hook(browser::read_url_state);
    let requested = initial.id.clone();
    let landing = landing_story(registry, requested.as_deref());

    let mut selected = use_signal(|| landing.clone());
    let mut args = use_signal(|| initial.args.clone());
    let mut query = use_signal(String::new);
    let mut collapsed = use_signal(BTreeSet::<String>::new);
    let mut cursor = use_signal(|| 0usize);
    let last_rendered = use_signal(|| None::<String>);
    let mut stale_link = use_signal(|| {
        requested
            .clone()
            .filter(|id| registry.get(id).is_none())
    });

    // Addon-panel state. The controls panel edits `args`; the two signals below
    // are what it reads to know where each control is currently sitting and
    // what the component has been up to.
    let initial_args = use_signal(ArgMap::new);
    let mut actions = use_signal(Vec::<ActionEntry>::new);
    let action_seq = use_signal(|| 0usize);
    let mut tab = use_signal(|| PanelTab::Controls);
    let mut panel_open = use_signal(|| true);

    // Listen to the preview's side of the conversation.
    // `Channel::subscribe` takes an `Fn`, so the signals are copied in rather
    // than captured by mutable reference — `Signal` is `Copy` for exactly this.
    use_hook(|| {
        Rc::new(channel.subscribe(Rc::new(move |event: &Event| {
            // `Signal` is `Copy`, so taking a fresh copy per call keeps this an
            // `Fn` — mutating a captured binding would demand `FnMut`.
            let (mut rendered, mut stale) = (last_rendered, stale_link);
            let (mut initial_args, mut args) = (initial_args, args);
            let (mut actions, mut seq) = (actions, action_seq);
            match event {
                Event::StoryRendered { id } => rendered.set(Some(id.clone())),
                Event::StoryMissing { id } => stale.set(Some(id.clone())),
                Event::StoryPrepared { initial_args: defaults, .. } => {
                    initial_args.set(defaults.clone());
                }
                Event::ActionLogged { name, payload } => {
                    let next = seq() + 1;
                    seq.set(next);
                    let mut log = actions.write();
                    log.push(ActionEntry {
                        seq: next,
                        name: name.clone(),
                        payload: payload.clone(),
                    });
                    // A component in a render loop can call a handler thousands
                    // of times; the panel is a tail, not an archive.
                    let overflow = log.len().saturating_sub(ACTION_LOG_CAP);
                    log.drain(..overflow);
                }
                // A story writing its own args back. Merge and let the effect
                // below re-broadcast the result, so the manager stays the single
                // place that decides what the current arg set is.
                Event::RequestArgsUpdate { args: partial, .. } => {
                    // `peek`, not a read: this runs inside a listener, and
                    // subscribing the manager's own effect to its own arg signal
                    // here would be a cycle rather than a dependency.
                    let current = args.peek().clone();
                    let merged = current.clone().overlay(partial);
                    // Guarded: the merge is re-broadcast as `UpdateArgs`, and an
                    // unconditional `set` would keep the effect firing forever.
                    if current != merged {
                        args.set(merged);
                    }
                }
                _ => {}
            }
        })))
    });

    // Selection and args flow out through the channel and into the URL. Both
    // happen in one effect so a link is never a render behind what is on screen.
    let publisher = channel.clone();
    use_effect(move || {
        let id = selected();
        let current_args = args();
        browser::write_url_state(&UrlState {
            id: id.clone(),
            args: current_args.clone(),
        });
        if let Some(id) = id {
            publisher.emit(Event::SetCurrentStory { id: id.clone() });
            publisher.emit(Event::UpdateArgs {
                id,
                args: current_args,
            });
        }
    });

    let rows = use_memo(move || {
        let text = query();
        let tree = if text.trim().is_empty() {
            registry.tree()
        } else {
            let hits: Vec<String> = registry.search(&text).iter().map(|s| s.id()).collect();
            registry.tree_filtered(move |s| hits.contains(&s.id()))
        };
        flatten(&tree, &collapsed.read())
    });

    // Selecting a story is the one place selection changes, so it also parks the
    // keyboard cursor on the matching row.
    let mut select_story = move |id: String| {
        stale_link.set(None);
        selected.set(Some(id.clone()));
        // Args are per-story overrides and the actions log is per-story history;
        // carrying either onto another story is never what the author meant.
        args.set(ArgMap::new());
        actions.write().clear();
        if let Some(i) = rows.read().iter().position(|r| matches!(&r.kind, RowKind::Story { id: rid, .. } if *rid == id)) {
            cursor.set(i);
        }
    };

    let mut toggle_group = move |path: String| {
        let mut set = collapsed.write();
        if !set.remove(&path) {
            set.insert(path);
        }
    };

    let mut move_cursor = move |delta: isize| {
        let len = rows.read().len();
        if len == 0 {
            return;
        }
        let next = (cursor() as isize + delta).clamp(0, len as isize - 1) as usize;
        cursor.set(next);
        // Arrowing through the tree previews as it goes, the way Storybook does.
        let landed = rows.read().get(next).map(|r| r.kind.clone());
        if let Some(RowKind::Story { id, .. }) = landed {
            stale_link.set(None);
            selected.set(Some(id));
            args.set(ArgMap::new());
            actions.write().clear();
        }
    };

    let current = selected().and_then(|id| registry.get(&id));
    let row_list = rows();
    // Clearing the overrides already tells the preview everything it needs, but
    // `ResetArgs` is emitted anyway: from M3 the preview holds state the manager
    // cannot see, and "go back to defaults" has to be a message, not an absence.
    let resetter = channel.clone();

    rsx! {
        style { {MANAGER_CSS} }
        div {
            class: "dxsb",
            tabindex: "0",
            onkeydown: move |event: KeyboardEvent| {
                match event.key() {
                    Key::ArrowDown => { event.prevent_default(); move_cursor(1); }
                    Key::ArrowUp => { event.prevent_default(); move_cursor(-1); }
                    Key::ArrowRight | Key::ArrowLeft => {
                        let want_open = event.key() == Key::ArrowRight;
                        let row = rows.read().get(cursor()).map(|r| r.kind.clone());
                        if let Some(RowKind::Group { path, expanded, .. }) = row {
                            if expanded != want_open {
                                event.prevent_default();
                                toggle_group(path);
                            }
                        }
                    }
                    Key::Enter => {
                        let row = rows.read().get(cursor()).map(|r| r.kind.clone());
                        match row {
                            Some(RowKind::Story { id, .. }) => select_story(id),
                            Some(RowKind::Group { path, .. }) => toggle_group(path),
                            None => {}
                        }
                    }
                    Key::Escape => {
                        query.set(String::new());
                    }
                    Key::Character(c) if c == "/" => {
                        event.prevent_default();
                        browser::focus(SEARCH_ID);
                    }
                    _ => {}
                }
            },

            aside { class: "dxsb-sidebar",
                div { class: "dxsb-brand",
                    span { "dioxus-storybook" }
                    span { class: "dxsb-badge", "M2" }
                }
                div { class: "dxsb-searchwrap",
                    input {
                        id: SEARCH_ID,
                        class: "dxsb-search",
                        r#type: "search",
                        placeholder: "Search stories  /",
                        value: "{query}",
                        oninput: move |e| {
                            query.set(e.value());
                            cursor.set(0);
                        },
                    }
                }
                nav { class: "dxsb-tree",
                    if row_list.is_empty() {
                        div { class: "dxsb-empty",
                            if registry.is_empty() {
                                "No stories registered. Check that build.rs calls "
                                code { "dioxus_storybook_build::index" }
                                " and that main.rs includes the generated registry."
                            } else {
                                "No stories match that search."
                            }
                        }
                    }
                    for (i, row) in row_list.iter().enumerate() {
                        SidebarRow {
                            key: "{row_key(row)}",
                            row: row.clone(),
                            index: i,
                            is_cursor: i == cursor(),
                            is_selected: matches!(&row.kind, RowKind::Story { id, .. } if Some(id.clone()) == selected()),
                            on_activate: move |kind: RowKind| {
                                cursor.set(i);
                                match kind {
                                    RowKind::Story { id, .. } => select_story(id),
                                    RowKind::Group { path, .. } => toggle_group(path),
                                }
                            },
                        }
                    }
                }
                div { class: "dxsb-hint",
                    kbd { "↑" } " " kbd { "↓" } " browse · "
                    kbd { "←" } " " kbd { "→" } " fold · "
                    kbd { "/" } " search"
                }
            }

            main { class: "dxsb-main",
                Toolbar { story: current }
                Preview { registry }
                AddonPanel {
                    arg_types: current.map(StoryDef::arg_types).unwrap_or(&[]),
                    initial: initial_args(),
                    overrides: args(),
                    actions: actions(),
                    tab: tab(),
                    open: panel_open(),
                    on_tab: move |next| { tab.set(next); panel_open.set(true); },
                    on_toggle: move |()| { let open = panel_open(); panel_open.set(!open); },
                    on_set: move |(name, value): (String, ArgValue)| {
                        // Straight into the manager's own signal — the effect
                        // above is what puts it on the channel and in the URL.
                        // The panel never talks to the preview itself.
                        args.write().set(name, value);
                    },
                    on_unset: move |name: String| { args.write().remove(&name); },
                    on_reset: move |()| {
                        args.set(ArgMap::new());
                        if let Some(id) = selected() {
                            resetter.emit(Event::ResetArgs { id });
                        }
                    },
                    on_clear: move |()| actions.write().clear(),
                }
                StatusBar {
                    registry,
                    rendered: last_rendered(),
                    stale: stale_link(),
                    arg_count: args().len(),
                }
            }
        }
    }
}

fn row_key(row: &Row) -> String {
    match &row.kind {
        RowKind::Group { path, .. } => format!("g:{path}"),
        RowKind::Story { id, .. } => format!("s:{id}"),
    }
}

#[component]
fn SidebarRow(
    row: Row,
    index: usize,
    is_cursor: bool,
    is_selected: bool,
    on_activate: EventHandler<RowKind>,
) -> Element {
    let _ = index;
    let indent = format!("padding-left:{}px", 8 + row.depth * 13);
    let mut class = String::from("dxsb-row");
    if is_cursor {
        class.push_str(" cursor");
    }
    if is_selected {
        class.push_str(" selected");
    }
    let kind = row.kind.clone();

    match &row.kind {
        RowKind::Group {
            name, expanded, ..
        } => {
            let caret = if *expanded { "▾" } else { "▸" };
            let class = format!("{class} dxsb-group");
            rsx! {
                button {
                    class: "{class}",
                    style: "{indent}",
                    onclick: move |_| on_activate.call(kind.clone()),
                    span { class: "dxsb-caret", "{caret}" }
                    span { "{name}" }
                }
            }
        }
        RowKind::Story { name, .. } => rsx! {
            button {
                class: "{class}",
                style: "{indent}",
                onclick: move |_| on_activate.call(kind.clone()),
                span { class: "dxsb-caret" }
                span { "{name}" }
            }
        },
    }
}

#[component]
fn Toolbar(story: Option<&'static StoryDef>) -> Element {
    let Some(story) = story else {
        return rsx! { header { class: "dxsb-toolbar", span { class: "dxsb-crumb", "No story selected" } } };
    };
    let id = story.id();
    rsx! {
        header { class: "dxsb-toolbar",
            span { class: "dxsb-crumb",
                for (i, segment) in story.title().split('/').enumerate() {
                    if i > 0 {
                        span { class: "sep", "/" }
                    }
                    span { "{segment}" }
                }
                span { class: "sep", "/" }
                span { "{story.name()}" }
            }
            span { class: "dxsb-spacer" }
            span { class: "dxsb-tagrow",
                for tag in story.tags().iter() {
                    span { class: "dxsb-tag", "{tag}" }
                }
            }
            span { class: "dxsb-id", "{id}" }
        }
    }
}

/// The preview harness.
///
/// It deliberately learns which story to render from the [`Channel`], not from
/// a prop: when M3 moves this component into an iframe, the only thing that
/// changes is the transport behind [`InProcessChannel`].
///
/// [`Channel`]: dioxus_storybook_core::Channel
#[component]
fn Preview(registry: Registry) -> Element {
    let channel: InProcessChannel = use_context();

    // Resolved here, not received as a prop: the preview owns its initial state
    // exactly as an iframe would, and the channel carries every change after.
    let startup = use_hook(browser::read_url_state);

    // Owned by the ROOT scope, not by this one, even though this component is
    // the only thing that reads them.
    //
    // A `use_signal` here would be owned by `Preview`, and the writer is the
    // channel listener below — which the manager invokes from *its* scope, an
    // ancestor. dioxus-signals warns about exactly that ("a Copy value ... used
    // in a scope which is not a descendant of the owning scope") because the
    // value can be dropped while an outer scope still holds it. Benign today,
    // since `Preview` never unmounts before `Storybook`, but it fired on every
    // keystroke and would bury a real warning.
    //
    // Rooting the ownership fixes it without moving the state up into the
    // manager, which would break the rule that the preview learns everything
    // over the channel.
    let (current, args) = use_hook(|| {
        (
            Signal::new_in_scope(landing_story(registry, startup.id.as_deref()), ScopeId::ROOT),
            Signal::new_in_scope(startup.args.clone(), ScopeId::ROOT),
        )
    });

    // The Subscription is parked in hook storage, so it lives exactly as long as
    // this component and unsubscribes on unmount.
    use_hook(|| {
        Rc::new(channel.subscribe(Rc::new(move |event: &Event| {
            let (mut current, mut args) = (current, args);
            match event {
                Event::SetCurrentStory { id } => current.set(Some(id.clone())),
                Event::UpdateArgs { args: incoming, .. } => args.set(incoming.clone()),
                Event::ResetArgs { .. } => args.set(ArgMap::new()),
                _ => {}
            }
        })))
    });

    // Only the failure case is reported from here; a story that resolves reports
    // itself from inside `StoryHost`, which is the scope that actually rendered it.
    let reporter = channel.clone();
    use_effect(move || {
        let Some(id) = current() else { return };
        if registry.get(&id).is_none() {
            reporter.emit(Event::StoryMissing { id });
        }
    });

    let story = current().and_then(|id| registry.get(&id));
    let overrides = args();

    match story {
        Some(def) => rsx! {
            section { class: "dxsb-canvas",
                // A keyed list of exactly one. That is not a flourish: Dioxus
                // only honours `key` when diffing a list, and what is needed
                // here is a *remount* when the story changes. Rendering story
                // bodies straight into this scope would put their hooks in the
                // preview's hook list, and two stories that call a different
                // number of hooks would then corrupt each other on a switch.
                for def in [def] {
                    StoryHost { key: "{def.id()}", story: def, args: overrides.clone() }
                }
            }
        },
        None => rsx! {
            section { class: "dxsb-canvas",
                div { class: "dxsb-blank",
                    if registry.is_empty() {
                        "No stories were registered. Add a "
                        code { "#[story]" }
                        " function under your story directory and rebuild."
                    } else {
                        "Select a story from the sidebar."
                    }
                }
            }
        },
    }
}

/// One story, in a scope of its own.
///
/// Everything here needs to happen *inside* the scope that renders the story,
/// which is why it is a component rather than a few lines in [`Preview`]:
///
/// - The story's hooks belong to this scope, and this scope is remounted when
///   the story changes, so two stories cannot corrupt each other's hook list.
/// - [`ActionSink`] and [`ArgsHandle`] are provided here, so a story reaches
///   them ambiently instead of every story signature growing two parameters.
/// - `base_args` evaluates the story's typed props, which builds `EventHandler`s
///   and therefore requires a live scope. It runs once per mount, in a
///   `use_hook` initialiser, and the answer goes to the manager over the channel
///   — the manager must not compute it, because from M3 the story functions live
///   in the other bundle.
#[component]
fn StoryHost(story: &'static StoryDef, args: ArgMap) -> Element {
    let channel: InProcessChannel = use_context();
    let id = story.id();

    let sink_channel = channel.clone();
    use_context_provider(|| {
        ActionSink::new(move |name: &str, payload: String| {
            sink_channel.emit(Event::ActionLogged {
                name: name.to_string(),
                payload,
            });
        })
    });

    let write_channel = channel.clone();
    let write_id = id.clone();
    use_context_provider(|| {
        ArgsHandle::new(move |name: String, value: ArgValue| {
            // A delta, not a replacement: the story knows the one arg it just
            // changed and nothing about the rest of the set.
            write_channel.emit(Event::RequestArgsUpdate {
                id: write_id.clone(),
                args: ArgMap::new().with(name, value),
            });
        })
    });

    let prepare_channel = channel.clone();
    let prepare_id = id.clone();
    use_hook(|| {
        prepare_channel.emit(Event::StoryPrepared {
            id: prepare_id,
            initial_args: story.base_args(),
        });
    });

    let reporter = channel.clone();
    use_effect(move || reporter.emit(Event::StoryRendered { id: id.clone() }));

    // The story body is a fn pointer invoked right here, inside a live Dioxus
    // scope, because `rsx!` and `EventHandler::new` need one.
    story.render(&args)
}

#[component]
fn StatusBar(
    registry: Registry,
    rendered: Option<String>,
    stale: Option<String>,
    arg_count: usize,
) -> Element {
    let count = registry.len();
    let rendered = rendered.unwrap_or_else(|| "—".into());
    rsx! {
        footer { class: "dxsb-status",
            span { "{count} stories" }
            span { "rendered: {rendered}" }
            span { "args: {arg_count}" }
            if let Some(id) = stale {
                span { class: "warn", "no such story: {id}" }
            }
        }
    }
}
