//! The dynamic-argument vocabulary.
//!
//! Storybook spreads a JSON object onto props. Rust needs the inverse, because
//! Dioxus props hold `EventHandler` and `Element`, which are not serialisable.
//! So the flow here is *overlay*, not deserialise: [`ArgMap`] carries untyped
//! values from the URL or the controls panel, and
//! [`Controllable::apply`] lays them over a typed base value, keeping the
//! story's own defaults for anything missing or unconvertible.

use std::collections::BTreeMap;

/// A single dynamic argument value, as it arrives from the URL or a control.
///
/// This deliberately has no type information beyond the shape: values coming
/// from a URL are untrusted text, and typing is restored by
/// [`Controllable::apply`], which knows the real field types.
///
/// Marked `#[non_exhaustive]`: more shapes (lists, objects, dates) arrive in
/// later milestones, so downstream `match`es need a wildcard arm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ArgValue {
    /// Free text.
    Text(String),
    /// Any number. Integers are narrowed on the way out by [`FromArg`].
    Num(f64),
    /// A boolean.
    Bool(bool),
    /// An enum variant, selected by name. See [`ControlEnum`].
    Variant(String),
    /// An ordered list, for `Vec<T>` fields.
    ///
    /// Homogeneous in practice but not by construction: like every other
    /// variant, typing is restored on the way out by [`FromArg`], which drops
    /// items it cannot convert rather than failing the whole list.
    List(Vec<ArgValue>),
    /// An explicitly absent value; converts to `None` for `Option<T>` fields.
    Null,
}

impl ArgValue {
    /// Render this value the way a text control should show it.
    ///
    /// Lists join with `", "`, which is exactly the form
    /// [`FromArg for Vec<T>`](FromArg) parses back, so a text widget round-trips
    /// a list without needing a list widget.
    ///
    /// ```
    /// # use dioxus_storybook_core::ArgValue;
    /// let list = ArgValue::List(vec![ArgValue::Text("a".into()), ArgValue::Num(2.0)]);
    /// assert_eq!(list.as_text(), "a, 2");
    /// assert_eq!(ArgValue::Bool(true).as_text(), "true");
    /// assert_eq!(ArgValue::Null.as_text(), "");
    /// ```
    pub fn as_text(&self) -> String {
        match self {
            ArgValue::Text(s) | ArgValue::Variant(s) => s.clone(),
            ArgValue::Num(n) => format_num(*n),
            ArgValue::Bool(b) => b.to_string(),
            ArgValue::List(items) => items
                .iter()
                .map(ArgValue::as_text)
                .collect::<Vec<_>>()
                .join(", "),
            ArgValue::Null => String::new(),
        }
    }

    /// Read this value as a number, for the number and range widgets.
    pub fn as_num(&self) -> Option<f64> {
        match self {
            ArgValue::Num(n) => Some(*n),
            ArgValue::Text(s) => s.parse().ok(),
            ArgValue::Bool(b) => Some(f64::from(u8::from(*b))),
            _ => None,
        }
    }

    /// Read this value as a boolean, for the toggle widget.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ArgValue::Bool(b) => Some(*b),
            ArgValue::Text(s) => s.parse().ok(),
            _ => None,
        }
    }
}

/// Format a float without a trailing `.0`, so `1.0` shows as `1`.
///
/// Shared with the URL codec so a value looks the same in a control as it does
/// in the address bar.
pub(crate) fn format_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// A bag of dynamic argument values, keyed by prop name.
///
/// Ordered, so that URL encoding is stable and two equal arg sets always
/// produce the same link.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArgMap(BTreeMap<String, ArgValue>);

impl ArgMap {
    /// An empty map.
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Insert a value, builder-style.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: ArgValue) -> Self {
        self.0.insert(key.into(), value);
        self
    }

    /// Insert a value in place.
    pub fn set(&mut self, key: impl Into<String>, value: ArgValue) {
        self.0.insert(key.into(), value);
    }

    /// Remove a value, returning it if it was present.
    pub fn remove(&mut self, key: &str) -> Option<ArgValue> {
        self.0.remove(key)
    }

    /// Look up a raw value.
    pub fn get(&self, key: &str) -> Option<&ArgValue> {
        self.0.get(key)
    }

    /// `true` if no args are set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of args set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterate over the args in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ArgValue)> {
        self.0.iter()
    }

    /// The typed read used by generated appliers: take the dynamic value if it
    /// is present *and* convertible, otherwise keep the story's typed default.
    ///
    /// Never panics — garbage in a URL degrades to the default rather than
    /// taking down the preview.
    pub fn get_or<T: FromArg>(&self, key: &str, default: T) -> T {
        self.0.get(key).and_then(T::from_arg).unwrap_or(default)
    }

    /// Overlay `other` on top of `self`, with `other` winning.
    #[must_use]
    pub fn overlay(mut self, other: &ArgMap) -> Self {
        for (k, v) in other.iter() {
            self.0.insert(k.clone(), v.clone());
        }
        self
    }
}

impl FromIterator<(String, ArgValue)> for ArgMap {
    fn from_iter<I: IntoIterator<Item = (String, ArgValue)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// Converting a dynamic value back into a typed field.
///
/// Implementations must be total and lossy-but-safe: return `None` rather than
/// panicking, because the input may be an arbitrary query string.
pub trait FromArg: Sized {
    /// Attempt the conversion. `None` means "keep the default".
    fn from_arg(value: &ArgValue) -> Option<Self>;
}

/// Converting a typed field into a dynamic value, to seed the controls panel
/// and the URL from a story's declared props.
pub trait ToArg {
    /// Produce the dynamic representation of this value.
    fn to_arg(&self) -> ArgValue;
}

macro_rules! num_impl {
    ($($t:ty),* $(,)?) => {$(
        impl FromArg for $t {
            fn from_arg(value: &ArgValue) -> Option<Self> {
                match value {
                    ArgValue::Num(n) => Some(*n as $t),
                    ArgValue::Text(s) => s.parse().ok(),
                    _ => None,
                }
            }
        }
        impl ToArg for $t {
            fn to_arg(&self) -> ArgValue { ArgValue::Num(*self as f64) }
        }
    )*};
}
num_impl!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

impl FromArg for String {
    fn from_arg(value: &ArgValue) -> Option<Self> {
        match value {
            ArgValue::Null => None,
            other => Some(other.as_text()),
        }
    }
}
impl ToArg for String {
    fn to_arg(&self) -> ArgValue {
        ArgValue::Text(self.clone())
    }
}

impl FromArg for bool {
    fn from_arg(value: &ArgValue) -> Option<Self> {
        match value {
            ArgValue::Bool(b) => Some(*b),
            ArgValue::Text(s) => s.parse().ok(),
            _ => None,
        }
    }
}
impl ToArg for bool {
    fn to_arg(&self) -> ArgValue {
        ArgValue::Bool(*self)
    }
}

impl<T: FromArg> FromArg for Option<T> {
    fn from_arg(value: &ArgValue) -> Option<Self> {
        match value {
            ArgValue::Null => Some(None),
            other => Some(T::from_arg(other)),
        }
    }
}
impl<T: ToArg> ToArg for Option<T> {
    fn to_arg(&self) -> ArgValue {
        match self {
            Some(v) => v.to_arg(),
            None => ArgValue::Null,
        }
    }
}

/// A `Vec<T>` accepts either a real [`ArgValue::List`] or a comma-separated
/// [`ArgValue::Text`], which is what the text widget and a hand-written URL
/// produce. Items that will not convert are dropped, so one bad element does
/// not discard the rest.
///
/// ```
/// # use dioxus_storybook_core::{ArgValue, FromArg};
/// let from_text = Vec::<u32>::from_arg(&ArgValue::Text("1, 2, 3".into()));
/// assert_eq!(from_text, Some(vec![1, 2, 3]));
///
/// let mixed = ArgValue::List(vec![ArgValue::Num(4.0), ArgValue::Bool(true)]);
/// assert_eq!(Vec::<u32>::from_arg(&mixed), Some(vec![4]));
/// ```
impl<T: FromArg> FromArg for Vec<T> {
    fn from_arg(value: &ArgValue) -> Option<Self> {
        match value {
            ArgValue::List(items) => Some(items.iter().filter_map(T::from_arg).collect()),
            ArgValue::Null => Some(Vec::new()),
            ArgValue::Text(s) if s.trim().is_empty() => Some(Vec::new()),
            ArgValue::Text(s) => Some(
                s.split(',')
                    .map(|part| ArgValue::Text(part.trim().to_string()))
                    .filter_map(|v| T::from_arg(&v))
                    .collect(),
            ),
            other => T::from_arg(other).map(|v| vec![v]),
        }
    }
}
impl<T: ToArg> ToArg for Vec<T> {
    fn to_arg(&self) -> ArgValue {
        ArgValue::List(self.iter().map(ToArg::to_arg).collect())
    }
}

/// Implemented by `#[derive(ControlEnum)]` on a unit-only enum; supplies the
/// variant names that become the options of a select or radio control.
pub trait ControlEnum: Sized {
    /// Every variant name, in declaration order.
    const VARIANTS: &'static [&'static str];
    /// Parse a variant by name.
    fn from_variant(name: &str) -> Option<Self>;
    /// The name of this variant.
    fn variant_name(&self) -> &'static str;
}

/// Implemented by `#[derive(Controls)]` on a props struct.
///
/// This is the answer to "there is no `react-docgen` for Rust": the derive sees
/// field names, types and doc comments statically, and the applier it emits is
/// checked by the compiler.
pub trait Controllable: Sized {
    /// One row per prop: type, docs, and which widget drives it.
    fn arg_types() -> &'static [crate::ArgType];
    /// Overlay dynamic args onto a typed base value.
    ///
    /// Fields with no control (event handlers, `Element`, `#[control(skip)]`)
    /// are carried through from `self` untouched.
    fn apply(&self, args: &ArgMap) -> Self;
    /// Seed a dynamic arg map from this typed value.
    fn to_args(&self) -> ArgMap;
    /// Replace every [`Control::Action`] field with a handler that reports the
    /// call to `sink` and then calls the story's own handler.
    ///
    /// This is the half of the args story that [`apply`](Controllable::apply)
    /// cannot do: an `EventHandler` is not a value the panel edits, it is a
    /// value the panel *observes*. Every other field is carried through
    /// unchanged.
    ///
    /// Must be called from inside a Dioxus scope — it builds `EventHandler`s.
    ///
    /// [`Control::Action`]: crate::Control::Action
    fn wire_actions(&self, sink: &crate::ActionSink) -> Self;
}
