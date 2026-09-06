//! The dioxus-storybook manager shell: sidebar, search, keyboard navigation,
//! the preview harness, and URL state.
//!
//! # Two documents, one bundle
//!
//! From M3 the story canvas lives in an **iframe**, so a component under test
//! cannot reach the shell's DOM, cannot inherit its CSS, and cannot take the
//! whole page down with it.
//!
//! The two halves are not two *bundles*, though. [`Storybook`] points its
//! iframe at the same URL it is serving from, with `?viewMode=preview` added,
//! and the copy that loads there sees the parameter and renders the canvas
//! alone. One `dx build`, one wasm binary, two documents. The alternative —
//! two crates and a hand-written `iframe.html` — costs a second compile of
//! everything, and buys isolation this already has. It would also cut the
//! manager off from the story registry, which it needs for the sidebar and for
//! the props tables; sharing a binary keeps [`ArgType`] a `&'static` on both
//! sides and keeps every message on the wire owned data.
//!
//! Nothing above the transport changed to make this work: the manager still
//! only emits [`Event`]s on a [`Channel`], and the preview still only
//! subscribes. That indirection, written in M1 when it looked like ceremony, is
//! why M3 is a new `Channel` implementation rather than a rewrite.
//!
//! Off `wasm32` there is no iframe to make, so the shell renders the preview
//! inline over an [`InProcessChannel`] and the whole thing stays assertable
//! from a host test.
//!
//! [`ArgType`]: dioxus_storybook_core::ArgType
//! [`Channel`]: dioxus_storybook_core::Channel
//! [`Event`]: dioxus_storybook_core::Event
//! [`InProcessChannel`]: dioxus_storybook_core::InProcessChannel

#![deny(missing_docs)]

mod browser;
mod panels;
mod style;
mod toolbar;

use std::collections::BTreeSet;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_storybook_core::viewport::{self, RESPONSIVE, ROTATED_GLOBAL, VIEWPORT_GLOBAL};
use dioxus_storybook_core::{
    ActionSink, ArgMap, ArgValue, ArgsHandle, Channel, ChannelHandle, Event, Project, Registry,
    Row, RowKind, StoryDef, UrlState, ViewMode, ViewportSelection, flatten,
};

use toolbar::{GlobalsBar, ViewportPicker};

use panels::{ACTION_LOG_CAP, ActionEntry, AddonPanel, PanelTab};

pub use style::{MANAGER_CSS, PREVIEW_CSS};

/// The id of the search input, so `/` can focus it.
const SEARCH_ID: &str = "dxsb-search";

/// The id of the preview iframe. The manager's transport looks the element up
/// by this id every time it sends, because the frame is replaced outright when
/// the preview reloads.
const PREVIEW_FRAME_ID: &str = "dxsb-preview-frame";

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

/// The storybook.
///
/// Mount it as your whole app:
///
/// ```ignore
/// fn main() {
///     dioxus::launch(|| rsx! { Storybook { registry: stories::registry() } });
/// }
/// ```
///
/// # It is two things
///
/// This one component is both halves. Which one it renders is decided by the
/// document's own URL: with `?viewMode=preview` it is the story canvas, alone;
/// without, it is the shell, and the shell points an iframe back at the same
/// URL *with* that parameter. See the [module docs](self).
///
/// The consequence worth knowing about: **a story renders in a document that
/// has none of your app's CSS or assets in it.** Components that carry their
/// own styles are unaffected; anything relying on a global stylesheet needs
/// that stylesheet in the preview document too.
#[component]
pub fn Storybook(registry: Registry, #[props(default)] project: Project) -> Element {
    // Both read once, in hooks. A document does not change roles while it is
    // running, and rebuilding the transport on every render would drop every
    // subscription with it.
    let mode = use_hook(browser::view_mode);
    let channel = use_hook(|| browser::channel_for(mode, PREVIEW_FRAME_ID));

    match mode {
        ViewMode::Preview => rsx! { StorybookPreview { registry, channel, project } },
        // `ViewMode` is `#[non_exhaustive]`, and an unrecognised mode is the
        // shell for the same reason a bare URL is.
        //
        // The manager takes the project too, but for one thing only: the list
        // of globals the toolbar has to offer. It must never *render* with it —
        // decorators wrap stories, and the shell renders none.
        _ => rsx! { StorybookManager { registry, channel, project } },
    }
}

/// The preview half on its own: the story canvas, and nothing around it.
///
/// [`Storybook`] mounts this for you in the framed document. Mount it yourself
/// only if you are supplying your own transport — a test harness pairing the
/// two halves, or an embedding that is not an iframe.
#[component]
pub fn StorybookPreview(
    registry: Registry,
    channel: ChannelHandle,
    #[props(default)] project: Project,
) -> Element {
    // Provided as context so `Preview` and `StoryHost` — and, later, a
    // preview-side addon — reach the bus without being handed it through every
    // layer.
    use_context_provider(|| channel);
    use_hook(browser::report_panics_to_manager);

    rsx! {
        style { {PREVIEW_CSS} }
        Preview { registry, project }
    }
}

/// The manager half on its own: the shell, and the preview either framed
/// (in a browser) or inline (anywhere else).
///
/// [`Storybook`] mounts this for you. See [`StorybookPreview`] for when you
/// would reach for it directly.
#[component]
pub fn StorybookManager(
    registry: Registry,
    channel: ChannelHandle,
    #[props(default)] project: Project,
) -> Element {
    // Provided as context so an addon panel — and the inline preview, where
    // there is one — talks to the bus without either half holding the other's
    // signals.
    use_context_provider(|| channel.clone());

    // The URL is the only state that survives a rebuild, so it is the source of
    // truth at startup, not a mirror written afterwards.
    let initial = use_hook(browser::read_url_state);
    let requested = initial.id.clone();
    let landing = landing_story(registry, requested.as_deref());

    let mut selected = use_signal(|| landing.clone());
    let mut args = use_signal(|| initial.args.clone());
    // The *selections* only. Declared defaults are filled in on the way out, by
    // both halves independently, because the declarations are `&'static` and
    // both halves share the bundle. Putting them on the wire or in the link
    // would only make both longer and let them go stale.
    let mut globals = use_signal(|| initial.globals.clone());
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

    // Bumped every time the preview announces itself. The publishing effect
    // below reads it, so a preview that mounts late — or reloads on its own,
    // which `dx serve` makes it do on every source edit — is resynced rather
    // than left showing whatever its URL said.
    let handshake = use_signal(|| 0usize);

    // Set once, and never cleared by anything but a reload: after a panic the
    // preview's wasm module is gone, so there is no "recovered" state to observe.
    let mut dead = use_signal(|| None::<String>);

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
            let (mut handshake, mut dead) = (handshake, dead);
            match event {
                Event::PreviewReady => {
                    // `peek`, not a read: this listener is not a reactive
                    // scope, and reading here would subscribe nothing anyway.
                    let next = handshake.peek().wrapping_add(1);
                    handshake.set(next);
                    // A fresh document is a live one: this is the only thing
                    // that clears a death notice, and it means the frame was
                    // reloaded.
                    dead.set(None);
                }
                Event::PreviewPanicked { message } => dead.set(Some(message.clone())),
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
        let current_globals = globals();
        // Read, not ignored: this is what subscribes the effect to the
        // handshake, so a preview that (re)mounts gets the current state.
        let _ = handshake();
        browser::write_url_state(&UrlState {
            id: id.clone(),
            args: current_args.clone(),
            globals: current_globals.clone(),
        });
        // Globals first, and outside the `if`: they are not about any one story,
        // and a book with no story selected still has a theme.
        publisher.emit(Event::SetGlobals {
            globals: current_globals,
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

    // Computed once, at mount. The iframe's `src` must not track live state:
    // changing it reloads the frame, and the whole point of the channel is that
    // it does not have to.
    let mut frame_src = use_signal(|| {
        browser::preview_src(&UrlState {
            id: landing.clone(),
            args: initial.args.clone(),
            globals: initial.globals.clone(),
        })
    });
    // Bumped only by the reload button. Changing `src` is what reloads an
    // iframe, which is precisely why nothing else may touch it.
    let mut reloads = use_signal(|| 0usize);

    let current = selected().and_then(|id| registry.get(&id));
    let row_list = rows();

    // The canvas size, decided by the same function the preview uses to fill in
    // `StoryContext::viewport` — so the frame the shell draws and the size the
    // story is told about cannot disagree.
    //
    // The story's parameters are part of the input, which is why this is
    // recomputed per render rather than kept in a signal: walking the sidebar
    // onto a story that asks for a phone must change the frame.
    let story_parameters = current
        .map(|def| def.resolved_parameters(project))
        .unwrap_or_default();
    let viewports = project.viewports();
    let viewport_selection = viewport::resolve(viewports, &globals(), story_parameters);
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
                    span { class: "dxsb-badge", "M3" }
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
                Toolbar {
                    story: current,
                    viewport: rsx! {
                        ViewportPicker {
                            available: viewports,
                            selection: viewport_selection,
                            on_pick: move |name: String| {
                                // Into the globals signal like any other
                                // selection: the effect above is what puts it
                                // on the wire and in the link.
                                let mut selections = globals.write();
                                if name == RESPONSIVE
                                    && !viewport::responsive_needs_saying(
                                        viewports,
                                        &selections,
                                        story_parameters,
                                    )
                                {
                                    // Nothing to override, so say nothing:
                                    // an absent key keeps the link short and
                                    // keeps "the map is empty" meaning
                                    // "nothing has been changed".
                                    selections.remove(VIEWPORT_GLOBAL);
                                    selections.remove(ROTATED_GLOBAL);
                                } else {
                                    selections.set(VIEWPORT_GLOBAL, ArgValue::Variant(name));
                                }
                            },
                            on_rotate: move |()| {
                                let mut selections = globals.write();
                                let rotated = selections
                                    .get(ROTATED_GLOBAL)
                                    .and_then(ArgValue::as_bool)
                                    .unwrap_or(false);
                                // Removed rather than set to false, so an
                                // unrotated canvas leaves nothing behind in
                                // the URL to explain.
                                if rotated {
                                    selections.remove(ROTATED_GLOBAL);
                                } else {
                                    selections.set(ROTATED_GLOBAL, ArgValue::Bool(true));
                                }
                            },
                        }
                    },
                    globals: rsx! {
                        GlobalsBar {
                            declared: project.globals(),
                            values: project.resolved_globals(&globals()),
                            changed: !globals().is_empty(),
                            on_set: move |(name, value): (String, ArgValue)| {
                                // Into the manager's own signal; the effect above
                                // is what puts it on the channel and in the URL.
                                //
                                // A value equal to the declared default is
                                // *removed* rather than stored: only real
                                // selections belong in a link, and it is what
                                // makes an empty map mean "nothing is changed".
                                let mut selections = globals.write();
                                if dioxus_storybook_core::globals::is_default(
                                    project.globals(),
                                    &name,
                                    &value,
                                ) {
                                    selections.remove(&name);
                                } else {
                                    selections.set(name, value);
                                }
                            },
                            on_reset: move |()| globals.set(ArgMap::new()),
                        }
                    },
                }
                if let Some(message) = dead() {
                    PreviewDied {
                        message,
                        on_reload: move |()| {
                            let next = reloads() + 1;
                            reloads.set(next);
                            let base = browser::preview_src(&UrlState {
                                id: selected(),
                                args: args(),
                                globals: globals(),
                            });
                            frame_src.set(format!("{base}&dxsb-reload={next}"));
                            dead.set(None);
                        },
                    }
                }
                PreviewStage { src: frame_src(), viewport: viewport_selection }
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

/// Keep the toolbar's own keystrokes out of the sidebar's keyboard navigation.
///
/// The shell listens for arrows, Enter and `/` on the whole `.dxsb` element,
/// which is what makes the sidebar browsable without the mouse. A `<select>` in
/// the toolbar wants exactly the same keys, and without this the two fight:
/// focus the viewport picker, press ↓, and the *story* changes underneath you
/// while the dropdown stays where it was.
///
/// Stopping propagation here rather than inspecting the event's target in the
/// outer handler keeps the rule where the exception is, and needs nothing from
/// the platform — there is no DOM to ask about a target off `wasm32`.
///
/// Found by opening the browser, not by the suite: a headless `VirtualDom` has
/// no way to deliver a keystroke at all.
fn swallow_toolbar_keys(event: KeyboardEvent) {
    event.stop_propagation();
}

#[component]
fn Toolbar(story: Option<&'static StoryDef>, viewport: Element, globals: Element) -> Element {
    let Some(story) = story else {
        return rsx! {
            header { class: "dxsb-toolbar", onkeydown: swallow_toolbar_keys,
                span { class: "dxsb-crumb", "No story selected" }
                span { class: "dxsb-spacer" }
                {viewport}
                {globals}
            }
        };
    };
    let id = story.id();
    rsx! {
        header { class: "dxsb-toolbar", onkeydown: swallow_toolbar_keys,
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
            {viewport}
            {globals}
            span { class: "dxsb-tagrow",
                for tag in story.tags().iter() {
                    span { class: "dxsb-tag", "{tag}" }
                }
            }
            span { class: "dxsb-id", "{id}" }
        }
    }
}

/// The notice shown when the story canvas has stopped.
///
/// This exists because of the split. One document per half means the shell can
/// keep working — sidebar, panels, search — while the thing it is a workbench
/// *for* is a frozen rectangle. Saying so, with the panic message and a way
/// back, is the difference between a bug and a mystery.
///
/// There is no automatic recovery to offer: `panic = "abort"` means the wasm
/// module is gone, not unwound, so the frame has to be reloaded.
#[component]
fn PreviewDied(message: String, on_reload: EventHandler<()>) -> Element {
    rsx! {
        div { class: "dxsb-died", role: "alert",
            div { class: "dxsb-died-head",
                strong { "The preview stopped." }
                button {
                    class: "dxsb-panelbtn",
                    onclick: move |_| on_reload.call(()),
                    "Reload the preview"
                }
            }
            pre { class: "dxsb-died-body", "{message}" }
        }
    }
}

/// The preview, in a document of its own.
///
/// `src` points back at this same app with `?viewMode=preview`, so the bundle
/// the browser already has is what loads inside — no second build, no second
/// HTML file, no dx configuration.
///
/// Deliberately **not** sandboxed. A `sandbox` attribute permissive enough to
/// run the story (`allow-scripts`) and let the two halves talk
/// (`allow-same-origin`) grants back everything it took away, so it would be
/// decoration. What the frame actually buys is a separate DOM, a separate CSS
/// cascade, a separate JS global scope and a viewport that can be resized
/// independently — isolation from *accident*, not from malice. The stories are
/// the developer's own code, compiled into the same binary as the shell.
#[component]
fn PreviewFrame(src: String, style: String) -> Element {
    rsx! {
        iframe {
            id: PREVIEW_FRAME_ID,
            class: "dxsb-frame",
            src: "{src}",
            style: "{style}",
            title: "Story preview",
        }
    }
}

/// The pane the frame sits in, and the viewport addon's entire preview-side
/// implementation.
///
/// A viewport is a width, the width belongs to the frame, and the frame belongs
/// to the shell — so "render this story on a phone" is a style attribute here
/// and nothing at all over there. The story is not told to re-render, is not
/// handed a breakpoint, and does not know it is being resized: its own media
/// queries fire because it genuinely is that size. That is the payoff of the
/// M3 split, and it is why this addon cost a stylesheet rule and a `<select>`.
///
/// Note what does *not* change: `src`. Changing an iframe's `src` reloads the
/// document inside it, which would throw away the preview's state and its
/// channel subscription on every resize. Only the style moves.
#[component]
fn PreviewStage(src: String, viewport: Option<ViewportSelection>) -> Element {
    let (class, style) = match viewport {
        Some(view) => (
            "dxsb-stage sized",
            format!("width:{}px;height:{}px", view.width(), view.height()),
        ),
        // No inline size at all, rather than `100%`: the stylesheet already
        // makes a responsive frame fill the pane, and an empty attribute is one
        // fewer thing overriding it.
        None => ("dxsb-stage", String::new()),
    };
    rsx! {
        div { class: "{class}",
            PreviewFrame { src, style }
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
fn Preview(registry: Registry, project: Project) -> Element {
    let channel: ChannelHandle = use_context();

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
    let (current, args, globals) = use_hook(|| {
        (
            Signal::new_in_scope(landing_story(registry, startup.id.as_deref()), ScopeId::ROOT),
            Signal::new_in_scope(startup.args.clone(), ScopeId::ROOT),
            // Read from this document's own URL, like everything else here: the
            // frame's `src` carries the globals so the first paint is already
            // themed, rather than flashing the default and correcting once the
            // handshake completes.
            Signal::new_in_scope(startup.globals.clone(), ScopeId::ROOT),
        )
    });

    // The Subscription is parked in hook storage, so it lives exactly as long as
    // this component and unsubscribes on unmount.
    use_hook(|| {
        Rc::new(channel.subscribe(Rc::new(move |event: &Event| {
            let (mut current, mut args, mut globals) = (current, args, globals);
            match event {
                Event::SetCurrentStory { id } => current.set(Some(id.clone())),
                Event::UpdateArgs { args: incoming, .. } => args.set(incoming.clone()),
                Event::SetGlobals { globals: incoming } => globals.set(incoming.clone()),
                Event::ResetArgs { .. } => args.set(ArgMap::new()),
                _ => {}
            }
        })))
    });

    // Announce, and only now: the subscription above has to exist first, because
    // the manager answers a handshake with the current story and args, and in a
    // synchronous transport that answer arrives before this hook returns.
    //
    // A hook rather than an effect, deliberately. Effects are a renderer's
    // business — they do not run under a bare `VirtualDom`, which is exactly the
    // arrangement `tests/two_documents.rs` uses to pair the two halves.
    let announcer = channel.clone();
    use_hook(|| announcer.emit(Event::PreviewReady));

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
    let selected_globals = globals();

    // The first thing to read a parameter. `layout` is the one Storybook's
    // canvas reads too, and it is the only decision about the *surface* a story
    // gets to make: whether the canvas centres it, stacks it at the top, or
    // hands over the whole frame. A decorator painting a theme needs the last
    // one, because 40px of canvas padding is 40px it cannot paint.
    let layout = story
        .map(|def| canvas_layout(def.resolved_parameters(project).str("layout")))
        .unwrap_or("centered");
    let canvas_class = format!("dxsb-canvas layout-{layout}");

    match story {
        Some(def) => rsx! {
            section { class: "{canvas_class}",
                // A keyed list of exactly one. That is not a flourish: Dioxus
                // only honours `key` when diffing a list, and what is needed
                // here is a *remount* when the story changes. Rendering story
                // bodies straight into this scope would put their hooks in the
                // preview's hook list, and two stories that call a different
                // number of hooks would then corrupt each other on a switch.
                for def in [def] {
                    StoryHost {
                        key: "{def.id()}",
                        story: def,
                        args: overrides.clone(),
                        globals: selected_globals.clone(),
                        project,
                    }
                }
            }
        },
        None => rsx! {
            section { class: "{canvas_class}",
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

/// The canvas layout a story asked for, or `"centered"` if it asked for
/// nothing this understands.
///
/// An unrecognised value is not an error: parameters are an open, stringly-typed
/// space shared with addons that this build may not have, and a story whose
/// `layout` is a typo should still render.
fn canvas_layout(requested: Option<&'static str>) -> &'static str {
    match requested {
        Some(layout @ ("centered" | "padded" | "fullscreen")) => layout,
        _ => "centered",
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
fn StoryHost(
    story: &'static StoryDef,
    args: ArgMap,
    globals: ArgMap,
    project: Project,
) -> Element {
    let channel: ChannelHandle = use_context();
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
    // scope, because `rsx!` and `EventHandler::new` need one — and so do the
    // decorators wrapped around it, which is the other reason this is a
    // component rather than a few lines in `Preview`.
    story.render_decorated(project, &args, &globals)
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
