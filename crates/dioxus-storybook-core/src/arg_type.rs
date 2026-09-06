//! Static metadata about one prop — the Rust analogue of Storybook's
//! `argTypes`, except produced by the compiler instead of by a docgen pass.

/// Which widget the controls panel renders for a prop.
///
/// Marked `#[non_exhaustive]`: the widget vocabulary grows in later milestones
/// (object, date, file), so downstream `match`es need a wildcard arm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Control {
    /// A single-line text field.
    Text,
    /// A numeric input, optionally bounded.
    Number {
        /// Lower bound, if any.
        min: Option<f64>,
        /// Upper bound, if any.
        max: Option<f64>,
        /// Increment, if any.
        step: Option<f64>,
    },
    /// A slider.
    Range {
        /// Lower bound.
        min: f64,
        /// Upper bound.
        max: f64,
        /// Increment.
        step: f64,
    },
    /// A checkbox.
    Toggle,
    /// A dropdown over enum variants.
    Select {
        /// The variant names.
        options: &'static [&'static str],
    },
    /// A radio group over enum variants.
    Radio {
        /// The variant names.
        options: &'static [&'static str],
    },
    /// A colour picker.
    Color,
    /// An `EventHandler` prop: no widget; calls are logged to the actions
    /// panel instead.
    Action,
    /// Explicitly excluded (`#[control(skip)]`) or an untranslatable type.
    None,
}

/// One row of the props table, and one control widget.
#[derive(Debug, Clone, PartialEq)]
pub struct ArgType {
    /// The prop's field name.
    pub name: &'static str,
    /// The prop's source type, rendered for the docs table.
    pub ty: &'static str,
    /// The text of the field's `///` doc comments.
    pub docs: &'static str,
    /// Which widget drives this prop.
    pub control: Control,
    /// `false` for `Option<T>` fields.
    pub required: bool,
}

impl ArgType {
    /// `true` if this prop participates in the dynamic arg overlay.
    ///
    /// Event handlers and skipped fields do not: they are carried through from
    /// the story's typed base value.
    pub fn is_dynamic(&self) -> bool {
        !matches!(self.control, Control::None | Control::Action)
    }
}
