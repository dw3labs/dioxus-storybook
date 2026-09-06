//! [`StoryDef`] and [`Meta`] — the two static descriptions the registry indexes.
//!
//! # Why fn pointers
//!
//! `StoryDef` stores `fn` pointers, never eagerly-evaluated values. `rsx!` and
//! `EventHandler::new` require an *active Dioxus scope*, not merely a runtime,
//! so a story's props and its rendered output can only be produced from inside
//! the preview component. Storing `fn() -> Props` and calling it there is what
//! makes that work. (Established by the M0 S3 spike; see `log/0002-m0-spikes.md`.)
//!
//! # Construction and semver
//!
//! Fields are private and values are built with `const` constructors:
//!
//! ```
//! # use dioxus_storybook_core::StoryDef;
//! # use dioxus_core::Element;
//! static PRIMARY: StoryDef = StoryDef::new("Forms/Button", "Primary", |_args| {
//!     // in real code, `rsx! { Button { ..props } }`
//!     # Ok(dioxus_core::VNode::placeholder())
//! });
//! ```
//!
//! That keeps adding an optional field a non-breaking change: it becomes a new
//! `with_*` method rather than a new struct field every caller must fill in.

use dioxus_core::Element;

use crate::{ArgMap, ArgType};

/// A `const`-constructible parameter value.
///
/// Parameters are static, non-serialisable addon configuration (viewport list,
/// background choices, docs options). They are deliberately *not* [`ArgValue`]:
/// args change at runtime, parameters do not.
///
/// [`ArgValue`]: crate::ArgValue
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ParamValue {
    /// A string.
    Str(&'static str),
    /// A number.
    Num(f64),
    /// A boolean.
    Bool(bool),
}

/// Static addon configuration attached to a project, a meta or a story.
///
/// One level's worth. The three levels are merged by [`ResolvedParameters`],
/// which is what an addon actually reads.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Parameters {
    entries: &'static [(&'static str, ParamValue)],
}

impl Parameters {
    /// An empty parameter set.
    pub const fn new() -> Self {
        Self { entries: &[] }
    }

    /// Build from a static table.
    pub const fn from_static(entries: &'static [(&'static str, ParamValue)]) -> Self {
        Self { entries }
    }

    /// Look up one parameter by key.
    pub fn get(&self, key: &str) -> Option<ParamValue> {
        self.entries
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
    }

    /// Iterate over every parameter.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, ParamValue)> + use<'_> {
        self.entries.iter().copied()
    }
}

/// Parameters as an addon sees them: the three levels, innermost first.
///
/// Merging is by key and the innermost level wins, so a story can override its
/// component, which can override the project. Nothing is allocated or copied to
/// do it — the levels are kept side by side and consulted in order, which also
/// means a resolved set stays `Copy` and `const`-friendly.
///
/// ```
/// # use dioxus_storybook_core::{ParamValue, Parameters, ResolvedParameters};
/// static PROJECT: &[(&str, ParamValue)] =
///     &[("layout", ParamValue::Str("padded")), ("theme", ParamValue::Str("light"))];
/// static STORY: &[(&str, ParamValue)] = &[("layout", ParamValue::Str("centered"))];
///
/// let resolved = ResolvedParameters::new(
///     Parameters::from_static(PROJECT),
///     Parameters::new(),
///     Parameters::from_static(STORY),
/// );
/// assert_eq!(resolved.get("layout"), Some(ParamValue::Str("centered")));
/// assert_eq!(resolved.get("theme"), Some(ParamValue::Str("light")));
/// assert_eq!(resolved.get("nothing"), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ResolvedParameters {
    project: Parameters,
    component: Parameters,
    story: Parameters,
}

impl ResolvedParameters {
    /// Stack three levels, outermost first.
    pub const fn new(project: Parameters, component: Parameters, story: Parameters) -> Self {
        Self { project, component, story }
    }

    /// The value for `key` from the innermost level that sets it.
    pub fn get(&self, key: &str) -> Option<ParamValue> {
        self.story
            .get(key)
            .or_else(|| self.component.get(key))
            .or_else(|| self.project.get(key))
    }

    /// The value for `key` if it is a string.
    pub fn str(&self, key: &str) -> Option<&'static str> {
        match self.get(key) {
            Some(ParamValue::Str(s)) => Some(s),
            _ => None,
        }
    }

    /// The value for `key` if it is a number.
    pub fn num(&self, key: &str) -> Option<f64> {
        match self.get(key) {
            Some(ParamValue::Num(n)) => Some(n),
            _ => None,
        }
    }

    /// The value for `key` if it is a boolean, or `default` if it is unset.
    ///
    /// A parameter of the wrong type also yields `default`: an addon reading a
    /// flag should not be able to be crashed by a typo three levels up.
    pub fn flag(&self, key: &str, default: bool) -> bool {
        match self.get(key) {
            Some(ParamValue::Bool(b)) => b,
            _ => default,
        }
    }

    /// Every key that is set anywhere, each with its winning value.
    ///
    /// Order is innermost level first, then the keys a level introduces.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, ParamValue)> + use<'_> {
        let mut seen: Vec<&'static str> = Vec::new();
        let levels = [self.story, self.component, self.project];
        let mut out: Vec<(&'static str, ParamValue)> = Vec::new();
        for level in levels {
            for (key, value) in level.iter() {
                if !seen.contains(&key) {
                    seen.push(key);
                    out.push((key, value));
                }
            }
        }
        out.into_iter()
    }
}

/// Wraps a story's rendered output.
///
/// A decorator is what you reach for when a story needs *surroundings* rather
/// than different props: a theme provider, a router, a fixed-width box, a
/// stylesheet. It receives the story's [`StoryContext`] and the element the
/// story produced, and returns whatever should be rendered instead.
///
/// ```
/// # use dioxus_storybook_core::{Decorator, StoryContext};
/// # use dioxus_core::Element;
/// static PADDED: Decorator = |_ctx: &StoryContext, story: Element| {
///     // in real code, `rsx! { div { style: "padding:2rem", {story} } }`
///     story
/// };
/// ```
///
/// # Where it runs
///
/// Inside the story's own scope, in the *preview* document. That is what makes
/// a decorator the answer to the question the M3 iframe raises — "how does my
/// app's stylesheet get into the frame?" — because a project-level decorator is
/// rendered in that document and can put a `document::Link` in it.
///
/// # Order
///
/// Project decorators are outermost, then the component's, then the story's.
/// Within one level, the first in the slice is the innermost, so reading a list
/// top to bottom walks inwards towards the story.
pub type Decorator = fn(&StoryContext, Element) -> Element;

/// What a decorator is told about the story it is wrapping.
///
/// `#[non_exhaustive]` and built only by this crate: it will grow globals and a
/// viewport later in M3, and growing it must not break a decorator.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct StoryContext {
    id: String,
    title: &'static str,
    name: &'static str,
    args: ArgMap,
    parameters: ResolvedParameters,
}

impl StoryContext {
    /// The story's stable identifier, `kebab(title)--kebab(name)`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The slash-separated sidebar path.
    pub const fn title(&self) -> &'static str {
        self.title
    }

    /// The story's display name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The args the story is being rendered with.
    pub const fn args(&self) -> &ArgMap {
        &self.args
    }

    /// Parameters, merged across project, component and story.
    pub const fn parameters(&self) -> ResolvedParameters {
        self.parameters
    }
}

/// Configuration that applies to every story in the book.
///
/// The outermost decorator layer and the bottom parameter layer — Storybook
/// calls these "project annotations" and keeps them in `preview.js`. Hand one to
/// [`Storybook`](https://docs.rs/dioxus-storybook/latest/dioxus_storybook/fn.Storybook.html):
///
/// ```
/// # use dioxus_storybook_core::{Decorator, Project, StoryContext};
/// # use dioxus_core::Element;
/// static DECORATORS: &[Decorator] = &[|_ctx: &StoryContext, story: Element| story];
/// static PROJECT: Project = Project::new().with_decorators(DECORATORS);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Project {
    decorators: &'static [Decorator],
    parameters: Parameters,
}

impl Project {
    /// No decorators, no parameters.
    pub const fn new() -> Self {
        Self {
            decorators: &[],
            parameters: Parameters::new(),
        }
    }

    /// Decorators wrapping every story, outside the component's and the story's.
    #[must_use]
    pub const fn with_decorators(mut self, decorators: &'static [Decorator]) -> Self {
        self.decorators = decorators;
        self
    }

    /// Parameters every story inherits unless it or its component says otherwise.
    #[must_use]
    pub const fn with_parameters(mut self, parameters: Parameters) -> Self {
        self.parameters = parameters;
        self
    }

    /// The project-level decorators.
    pub const fn decorators(&self) -> &'static [Decorator] {
        self.decorators
    }

    /// The project-level parameters.
    pub const fn parameters(&self) -> Parameters {
        self.parameters
    }
}

/// Component-level description shared by every story in a module.
///
/// Produced by the `story_meta!` macro; read by the `#[story]` macro to fill in
/// each story's title and tags.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Meta {
    title: &'static str,
    component: &'static str,
    tags: &'static [&'static str],
    parameters: Parameters,
    decorators: &'static [Decorator],
}

impl Meta {
    /// A meta with a sidebar `title` (slash-separated) and a component name.
    pub const fn new(title: &'static str, component: &'static str) -> Self {
        Self {
            title,
            component,
            tags: &[],
            parameters: Parameters::new(),
            decorators: &[],
        }
    }

    /// Attach tags inherited by every story in the module.
    #[must_use]
    pub const fn with_tags(mut self, tags: &'static [&'static str]) -> Self {
        self.tags = tags;
        self
    }

    /// Attach parameters inherited by every story in the module.
    #[must_use]
    pub const fn with_parameters(mut self, parameters: Parameters) -> Self {
        self.parameters = parameters;
        self
    }

    /// Attach decorators wrapping every story in the module.
    #[must_use]
    pub const fn with_decorators(mut self, decorators: &'static [Decorator]) -> Self {
        self.decorators = decorators;
        self
    }

    /// The slash-separated sidebar path, e.g. `"Forms/Button"`.
    pub const fn title(&self) -> &'static str {
        self.title
    }

    /// Decorators inherited by every story in the module.
    pub const fn decorators(&self) -> &'static [Decorator] {
        self.decorators
    }

    /// The component's name, for the docs page.
    pub const fn component(&self) -> &'static str {
        self.component
    }

    /// Tags inherited by every story in the module.
    pub const fn tags(&self) -> &'static [&'static str] {
        self.tags
    }

    /// Parameters inherited by every story in the module.
    pub const fn parameters(&self) -> Parameters {
        self.parameters
    }
}

/// One story: a named, renderable state of a component.
///
/// See the [module docs](self) for why every payload is a `fn` pointer and why
/// the fields are private.
pub struct StoryDef {
    title: &'static str,
    name: &'static str,
    render: fn(&ArgMap) -> Element,
    arg_types: fn() -> &'static [ArgType],
    base_args: fn() -> ArgMap,
    tags: &'static [&'static str],
    parameters: Parameters,
    decorators: &'static [Decorator],
    /// The component-level metadata this story inherited.
    ///
    /// Kept whole rather than copied field by field, so that every later
    /// addition to `Meta` reaches stories without `StoryDef` growing a field
    /// and the macro growing a line. `title` and `tags` are the exception: they
    /// are already merged by `#[story]` and stored resolved.
    meta: Meta,
}

impl StoryDef {
    /// A story with a sidebar `title`, a display `name`, and a render function.
    ///
    /// `render` is invoked from inside the preview component, so it may use
    /// `rsx!` and construct `EventHandler`s.
    pub const fn new(
        title: &'static str,
        name: &'static str,
        render: fn(&ArgMap) -> Element,
    ) -> Self {
        Self {
            title,
            name,
            render,
            arg_types: no_arg_types,
            base_args: ArgMap::new,
            tags: &[],
            parameters: Parameters::new(),
            decorators: &[],
            meta: Meta::new(title, ""),
        }
    }

    /// Attach the props table, normally `<P as Controllable>::arg_types`.
    #[must_use]
    pub const fn with_arg_types(mut self, arg_types: fn() -> &'static [ArgType]) -> Self {
        self.arg_types = arg_types;
        self
    }

    /// Attach the story's default args, normally derived from its typed props.
    #[must_use]
    pub const fn with_base_args(mut self, base_args: fn() -> ArgMap) -> Self {
        self.base_args = base_args;
        self
    }

    /// Attach tags.
    #[must_use]
    pub const fn with_tags(mut self, tags: &'static [&'static str]) -> Self {
        self.tags = tags;
        self
    }

    /// Attach parameters.
    #[must_use]
    pub const fn with_parameters(mut self, parameters: Parameters) -> Self {
        self.parameters = parameters;
        self
    }

    /// Attach decorators wrapping only this story, inside its component's.
    #[must_use]
    pub const fn with_decorators(mut self, decorators: &'static [Decorator]) -> Self {
        self.decorators = decorators;
        self
    }

    /// Attach the component-level metadata this story belongs to.
    ///
    /// `#[story]` does this for you; it is how a story reaches the parameters
    /// and decorators declared once in `story_meta!`.
    #[must_use]
    pub const fn with_meta(mut self, meta: Meta) -> Self {
        self.meta = meta;
        self
    }

    /// The component-level metadata this story inherited.
    pub const fn meta(&self) -> Meta {
        self.meta
    }

    /// Decorators declared on this story alone.
    pub const fn decorators(&self) -> &'static [Decorator] {
        self.decorators
    }

    /// The slash-separated sidebar path, e.g. `"Forms/Button"`.
    pub const fn title(&self) -> &'static str {
        self.title
    }

    /// The story's display name, e.g. `"Primary"`.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Tags on this story.
    pub const fn tags(&self) -> &'static [&'static str] {
        self.tags
    }

    /// Parameters on this story.
    pub const fn parameters(&self) -> Parameters {
        self.parameters
    }

    /// `true` if this story carries `tag`.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| *t == tag)
    }

    /// The stable, URL-safe identifier: `kebab(title)--kebab(name)`.
    ///
    /// ```
    /// # use dioxus_storybook_core::StoryDef;
    /// # let s = StoryDef::new("Forms/Button", "With Icon", |_| Ok(dioxus_core::VNode::placeholder()));
    /// assert_eq!(s.id(), "forms-button--with-icon");
    /// ```
    pub fn id(&self) -> String {
        format!("{}--{}", kebab(self.title), kebab(self.name))
    }

    /// The props table for this story.
    pub fn arg_types(&self) -> &'static [ArgType] {
        (self.arg_types)()
    }

    /// The story's own default args, before any URL or panel overlay.
    ///
    /// Like [`StoryDef::render`], this must be called from inside a Dioxus
    /// scope: it evaluates the story's props, and those props may build an
    /// `EventHandler`, which panics without one.
    pub fn base_args(&self) -> ArgMap {
        (self.base_args)()
    }

    /// Render the story with `args` overlaid on its defaults.
    ///
    /// Must be called from inside a Dioxus scope. This is the bare story;
    /// [`render_decorated`](Self::render_decorated) is what the preview calls.
    pub fn render(&self, args: &ArgMap) -> Element {
        (self.render)(args)
    }

    /// Parameters for this story, merged across project, component and story.
    pub const fn resolved_parameters(&self, project: Project) -> ResolvedParameters {
        ResolvedParameters::new(project.parameters(), self.meta.parameters(), self.parameters)
    }

    /// What a decorator is told about this story.
    pub fn context(&self, project: Project, args: &ArgMap) -> StoryContext {
        StoryContext {
            id: self.id(),
            title: self.title,
            name: self.name,
            args: args.clone(),
            parameters: self.resolved_parameters(project),
        }
    }

    /// Render the story inside its decorators: story's, then its component's,
    /// then the project's.
    ///
    /// The fold runs outwards, so the *last* decorator applied is the outermost
    /// element — which is why the chain is ordered innermost-level first.
    ///
    /// Must be called from inside a Dioxus scope: a decorator may use `rsx!`,
    /// and so may the story.
    pub fn render_decorated(&self, project: Project, args: &ArgMap) -> Element {
        let chain = self
            .decorators
            .iter()
            .chain(self.meta.decorators())
            .chain(project.decorators());
        let context = self.context(project, args);
        let mut element = self.render(args);
        for decorate in chain {
            element = decorate(&context, element);
        }
        element
    }
}

/// Two `StoryDef`s are the same story when they are the same static.
///
/// Needed because a `&'static StoryDef` travels as a Dioxus prop, and props are
/// compared to decide whether to re-render.
impl PartialEq for StoryDef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl core::fmt::Debug for StoryDef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StoryDef")
            .field("id", &self.id())
            .field("title", &self.title)
            .field("name", &self.name)
            .field("tags", &self.tags)
            .finish_non_exhaustive()
    }
}

fn no_arg_types() -> &'static [ArgType] {
    &[]
}

/// Lower-case a string and collapse every run of non-alphanumeric characters
/// into a single `-`, as Storybook does when it builds a story id.
///
/// ```
/// # use dioxus_storybook_core::kebab;
/// assert_eq!(kebab("Forms/Button"), "forms-button");
/// assert_eq!(kebab("  With   Icon! "), "with-icon");
/// ```
pub fn kebab(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pending_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(ch.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    out
}
