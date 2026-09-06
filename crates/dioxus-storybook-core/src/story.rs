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

/// Static addon configuration attached to a meta or a story.
///
/// M1 stores parameters but does not yet merge them across the
/// global → component → story levels; that lands with decorators in M3.
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
}

impl Meta {
    /// A meta with a sidebar `title` (slash-separated) and a component name.
    pub const fn new(title: &'static str, component: &'static str) -> Self {
        Self {
            title,
            component,
            tags: &[],
            parameters: Parameters::new(),
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

    /// The slash-separated sidebar path, e.g. `"Forms/Button"`.
    pub const fn title(&self) -> &'static str {
        self.title
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
    /// Must be called from inside a Dioxus scope.
    pub fn render(&self, args: &ArgMap) -> Element {
        (self.render)(args)
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
