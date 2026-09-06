//! Autodocs: the generated documentation page for one component.
//!
//! A docs page is not a story and it is not a panel. It is a second *entry* in
//! the same id space the sidebar and the URL already use — `forms-button--docs`
//! sits beside `forms-button--primary` — which is what makes it deep-linkable,
//! selectable and shareable with no new URL parameter and no new wire message.
//!
//! # Where it renders, and why that is not the manager
//!
//! In the **preview** document, with the stories on it rendered by the same
//! [`StoryDef::render_decorated`] the canvas uses. That is forced, not chosen:
//! evaluating a story's props builds `EventHandler`s and needs a live Dioxus
//! scope, decorators are what put the author's stylesheet inside the frame, and
//! from M3 the manager is a different document that must do neither. A docs
//! page whose examples were drawn by the manager would be a page of components
//! rendered under the *shell's* CSS — which is the one thing the M3 split
//! exists to prevent.
//!
//! # What is on it, and where each part comes from
//!
//! | | source |
//! |---|---|
//! | component name | `story_meta! { component: .. }` |
//! | description | `story_meta! { description: ".." }`, else the props type's `///` |
//! | props table | `#[derive(Controls)]` — names, types, `///` docs |
//! | per-story blurb | the `#[story]` function's own `///` |
//! | per-story source | the `#[story]` function's body, as written |
//!
//! Every one of them is read off the type by a macro. There is no docgen pass,
//! no sidecar file and no second place to keep a component's documentation up
//! to date.
//!
//! [`StoryDef::render_decorated`]: crate::StoryDef::render_decorated

use crate::registry::Registry;
use crate::story::{Project, StoryDef, kebab};

/// The tag that opts a component in under [`Autodocs::Tagged`].
pub const AUTODOCS_TAG: &str = "autodocs";

/// The suffix that turns a component's slug into its docs entry id.
pub const DOCS_SUFFIX: &str = "--docs";

/// Which components get a generated docs page.
///
/// `#[non_exhaustive]`: a later milestone may add a mode that reads a parameter
/// rather than a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Autodocs {
    /// Every component with at least one story. The default, because a docs
    /// page that has to be asked for is a docs page nobody has.
    #[default]
    Always,
    /// Only components whose `story_meta!` carries [`AUTODOCS_TAG`]. This is
    /// Storybook's own default, and the right answer for a book where most
    /// components are internal.
    Tagged,
    /// None. Removes the toolbar's Docs toggle entirely.
    Never,
}

/// The id of the docs entry for a sidebar `title`.
///
/// ```
/// # use dioxus_storybook_core::docs;
/// assert_eq!(docs::id_for("Forms/Button"), "forms-button--docs");
/// ```
pub fn id_for(title: &str) -> String {
    format!("{}{DOCS_SUFFIX}", kebab(title))
}

/// Whether a component with these `tags` gets a docs page under `mode`.
///
/// The tags are the *component's* — `story_meta! { tags: [..] }` — which every
/// story in the module inherits, so asking a story is the same as asking its
/// component.
pub fn enabled(mode: Autodocs, tags: &[&'static str]) -> bool {
    match mode {
        Autodocs::Always => true,
        Autodocs::Tagged => tags.contains(&AUTODOCS_TAG),
        Autodocs::Never => false,
    }
}

/// Everything one generated docs page shows.
///
/// Built by [`resolve`]; there is deliberately no public constructor, because
/// the invariant that `stories` is non-empty is what lets the page assume it
/// has a component to describe.
#[derive(Debug, Clone, PartialEq)]
pub struct DocsPage {
    title: &'static str,
    component: &'static str,
    description: &'static str,
    stories: Vec<&'static StoryDef>,
}

impl DocsPage {
    /// The slash-separated sidebar path this page documents, e.g. `Forms/Button`.
    pub fn title(&self) -> &'static str {
        self.title
    }

    /// The component's name, from `story_meta! { component: .. }`.
    pub fn component(&self) -> &'static str {
        self.component
    }

    /// The prose blurb, or `""` when the author wrote none.
    ///
    /// `story_meta! { description: ".." }` if it is set, otherwise the `///`
    /// doc comment on the props type, which `#[derive(Controls)]` captures.
    pub fn description(&self) -> &'static str {
        self.description
    }

    /// Every story under this title, in registry order.
    ///
    /// Never empty: a title with no stories has no docs page.
    pub fn stories(&self) -> &[&'static StoryDef] {
        &self.stories
    }

    /// The story whose props table describes the component.
    ///
    /// The first one that has a props table at all — an `Element`-form story
    /// has none, and a module that opens with one should still get a table.
    pub fn primary(&self) -> Option<&'static StoryDef> {
        self.stories
            .iter()
            .copied()
            .find(|s| !s.arg_types().is_empty())
    }

    /// This page's entry id.
    pub fn id(&self) -> String {
        id_for(self.title)
    }
}

/// What an id in the URL names.
///
/// Stories and docs pages share one id space, so the shell, the URL and the
/// wire all carry a single `id` and this is what reads it.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// One story, on the canvas.
    Story(&'static StoryDef),
    /// One component's generated documentation.
    Docs(DocsPage),
}

impl Entry {
    /// The title this entry belongs under.
    pub fn title(&self) -> &'static str {
        match self {
            Entry::Story(def) => def.title(),
            Entry::Docs(page) => page.title(),
        }
    }

    /// The story, if this entry is one.
    pub fn story(&self) -> Option<&'static StoryDef> {
        match self {
            Entry::Story(def) => Some(def),
            Entry::Docs(_) => None,
        }
    }

    /// The docs page, if this entry is one.
    pub fn docs(&self) -> Option<&DocsPage> {
        match self {
            Entry::Docs(page) => Some(page),
            Entry::Story(_) => None,
        }
    }
}

/// Build the docs page `id` names, or `None` if there is no such page.
///
/// `None` covers all three ways it can fail: the id is not a docs id, no
/// component has that slug, or the component is not opted in under
/// [`Project::autodocs`].
pub fn resolve(registry: Registry, project: Project, id: &str) -> Option<DocsPage> {
    let slug = id.strip_suffix(DOCS_SUFFIX)?;
    let stories: Vec<&'static StoryDef> = registry
        .stories()
        .iter()
        .copied()
        .filter(|def| kebab(def.title()) == slug)
        .collect();
    let first = stories.first().copied()?;
    if !enabled(project.autodocs(), first.meta().tags()) {
        return None;
    }
    // `story_meta!`'s own words win; the props type's `///` is the fallback,
    // and is what makes a docs page appear with no authoring at all.
    let described = stories
        .iter()
        .map(|s| s.meta().description())
        .find(|d| !d.is_empty())
        .or_else(|| {
            stories
                .iter()
                .map(|s| s.component_docs())
                .find(|d| !d.is_empty())
        })
        .unwrap_or("");
    Some(DocsPage {
        title: first.title(),
        component: first.meta().component(),
        description: described,
        stories,
    })
}

/// Resolve an id to whatever it names.
///
/// A **story wins** over a docs page with the same id. That matters because
/// `--docs` is a legal story id: a story literally named "Docs" collides, and
/// the author's own story is the one that should win.
pub fn entry(registry: Registry, project: Project, id: &str) -> Option<Entry> {
    if let Some(def) = registry.get(id) {
        return Some(Entry::Story(def));
    }
    resolve(registry, project, id).map(Entry::Docs)
}

/// The docs id for a story's component, if that component has a docs page.
///
/// This is what the toolbar's Canvas/Docs toggle asks: it is `Some` exactly
/// when the toggle should be drawn.
pub fn id_for_story(project: Project, def: &StoryDef) -> Option<String> {
    enabled(project.autodocs(), def.meta().tags()).then(|| id_for(def.title()))
}

/// The story a docs page's Canvas button should land on: the first under that
/// title.
pub fn canvas_id_for(registry: Registry, title: &str) -> Option<String> {
    registry
        .stories()
        .iter()
        .copied()
        .find(|def| def.title() == title)
        .map(StoryDef::id)
}
