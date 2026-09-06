//! Globals: the values the toolbar selects and every story sees.
//!
//! A global is the answer to "render every story in dark mode", or "in French",
//! or "right to left". It is not a prop — no story declares it, changing it does
//! not change which story you are looking at, and it survives moving between
//! stories. That last property is the whole point: flipping the theme and then
//! walking the sidebar is how you find the component that forgot about it.
//!
//! # Three vocabularies, deliberately not four
//!
//! | | changes at run time | scope | declared by |
//! |---|---|---|---|
//! | [`ArgValue`] args | yes | one story | the props type |
//! | [`ParamValue`] parameters | no | a level | project / meta / story |
//! | **globals** | yes | the whole book | the project |
//!
//! Globals reuse the *args* machinery rather than growing a fourth one: a
//! declaration is a [`Control`] exactly as a prop's is, and the current values
//! are an [`ArgMap`]. That is not an economy — it is what gives globals a
//! controls widget, a URL encoding and a wire encoding without a line of new
//! code in any of the three.

use crate::{ArgMap, ArgValue, Control, ParamValue};

/// One toolbar-selectable value that every story can see.
///
/// Declared on the [`Project`](crate::Project) and read by decorators through
/// [`StoryContext::globals`](crate::StoryContext::globals):
///
/// ```
/// # use dioxus_storybook_core::{GlobalType, Project};
/// static GLOBALS: &[GlobalType] = &[
///     GlobalType::select("theme", &["light", "dark"]).with_title("Theme"),
///     GlobalType::toggle("rtl", false).with_title("RTL"),
/// ];
/// static PROJECT: Project = Project::new().with_globals(GLOBALS);
/// ```
///
/// Fields are private and values are built with `const` constructors, so adding
/// one later is a new `with_*` method rather than a break for every caller.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalType {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    control: Control,
    default: ParamValue,
}

impl GlobalType {
    /// A global with an explicit default and a plain text control.
    pub const fn new(name: &'static str, default: ParamValue) -> Self {
        Self {
            name,
            title: name,
            description: "",
            control: Control::Text,
            default,
        }
    }

    /// A global chosen from a fixed list. The first option is the default.
    ///
    /// An empty list yields an empty default rather than a panic — a toolbar
    /// with nothing in it is a mistake worth seeing on screen, not a crash on
    /// the way to seeing it.
    pub const fn select(name: &'static str, options: &'static [&'static str]) -> Self {
        let default = if options.is_empty() {
            ParamValue::Str("")
        } else {
            ParamValue::Str(options[0])
        };
        Self {
            name,
            title: name,
            description: "",
            control: Control::Select { options },
            default,
        }
    }

    /// An on/off global.
    pub const fn toggle(name: &'static str, default: bool) -> Self {
        Self {
            name,
            title: name,
            description: "",
            control: Control::Toggle,
            default: ParamValue::Bool(default),
        }
    }

    /// The label the toolbar shows. Defaults to the name.
    #[must_use]
    pub const fn with_title(mut self, title: &'static str) -> Self {
        self.title = title;
        self
    }

    /// Help text, shown as the control's tooltip.
    #[must_use]
    pub const fn with_description(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }

    /// Use a different widget than the constructor chose.
    #[must_use]
    pub const fn with_control(mut self, control: Control) -> Self {
        self.control = control;
        self
    }

    /// Override the default value.
    #[must_use]
    pub const fn with_default(mut self, default: ParamValue) -> Self {
        self.default = default;
        self
    }

    /// The key this global is stored under, in the URL and on the wire.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The toolbar label.
    pub const fn title(&self) -> &'static str {
        self.title
    }

    /// The help text, or `""`.
    pub const fn description(&self) -> &'static str {
        self.description
    }

    /// The widget the toolbar renders.
    pub const fn control(&self) -> &Control {
        &self.control
    }

    /// The value in force when nothing has been selected.
    ///
    /// No wildcard arm: `ParamValue` is `#[non_exhaustive]` for downstream
    /// crates but exhaustive in here, so a new parameter shape fails to compile
    /// until it is given a dynamic equivalent.
    pub fn default_value(&self) -> ArgValue {
        match self.default {
            // `Variant`, not `Text`, when the control is a fixed list: that is
            // the shape a `#[derive(ControlEnum)]` prop uses for the same thing,
            // so a decorator matching on one matches on the other.
            ParamValue::Str(s) => match self.control {
                Control::Select { .. } | Control::Radio { .. } => ArgValue::Variant(s.into()),
                _ => ArgValue::Text(s.into()),
            },
            ParamValue::Num(n) => ArgValue::Num(n),
            ParamValue::Bool(b) => ArgValue::Bool(b),
        }
    }

    /// Force `value` into the shape this global's control implies.
    ///
    /// The same job [`Controllable::apply`](crate::Controllable::apply) does for
    /// props, and for the same reason: a value that came from a URL is untyped
    /// text. `?globals=theme:dark` decodes as [`ArgValue::Text`], but a
    /// decorator matching a `Select` global should be able to expect
    /// [`ArgValue::Variant`] — the shape its declaration promised.
    ///
    /// Anything unconvertible falls back to the declared default rather than
    /// failing, because the input is a link somebody pasted.
    pub fn coerce(&self, value: ArgValue) -> ArgValue {
        match self.control {
            Control::Select { .. } | Control::Radio { .. } => {
                ArgValue::Variant(value.as_text())
            }
            Control::Toggle => match value.as_bool() {
                Some(b) => ArgValue::Bool(b),
                None => self.default_value(),
            },
            Control::Number { .. } | Control::Range { .. } => match value.as_num() {
                Some(n) => ArgValue::Num(n),
                None => self.default_value(),
            },
            Control::Text | Control::Color => ArgValue::Text(value.as_text()),
            // A global has no reason to be an action or to be skipped, but the
            // function has to be total, so the value passes through.
            Control::Action | Control::None => value,
        }
    }
}

/// Fill in every declared global that `selected` does not set, and force what it
/// does set into the shape its declaration promised.
///
/// The result is total over the declarations: a decorator reading
/// `globals.get("theme")` never has to wonder whether the toolbar has been
/// touched yet, nor whether the value arrived from a link as raw text.
///
/// Names that are not declared are carried through untouched and uncoerced — a
/// stale link naming a global that has since been removed carries a value
/// nothing reads, which is not an error worth failing a page load over.
pub fn resolve(declared: &[GlobalType], selected: &ArgMap) -> ArgMap {
    let mut resolved = ArgMap::new();
    for global in declared {
        resolved.set(global.name(), global.default_value());
    }
    for (name, value) in selected.iter() {
        match declared.iter().find(|g| g.name() == name) {
            Some(global) => resolved.set(name, global.coerce(value.clone())),
            None => resolved.set(name, value.clone()),
        }
    }
    resolved
}

/// Whether `value` is simply what `name` would have been anyway.
///
/// Used to keep a selection out of the URL and off the wire when it matches the
/// declared default. Two reasons that matters more than tidiness: a link stays
/// short and stops carrying values nobody chose, and "has anything been changed"
/// — which is what a reset button is asking — becomes a question the selection
/// map can answer by being empty.
///
/// A name that is not declared is never "default", because there is nothing to
/// compare it to.
pub fn is_default(declared: &[GlobalType], name: &str, value: &ArgValue) -> bool {
    declared
        .iter()
        .find(|g| g.name() == name)
        .is_some_and(|g| g.coerce(value.clone()) == g.default_value())
}
