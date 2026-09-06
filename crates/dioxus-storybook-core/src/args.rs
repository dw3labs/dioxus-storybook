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
    /// An explicitly absent value; converts to `None` for `Option<T>` fields.
    Null,
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
            ArgValue::Text(s) | ArgValue::Variant(s) => Some(s.clone()),
            ArgValue::Num(n) => Some(n.to_string()),
            ArgValue::Bool(b) => Some(b.to_string()),
            ArgValue::Null => None,
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
}
