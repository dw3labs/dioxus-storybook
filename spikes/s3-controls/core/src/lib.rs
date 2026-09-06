//! S3 spike: the arg-type/control vocabulary a `#[derive(Controls)]` must emit,
//! plus the dynamic-args -> typed-props applier (problem P3 in the plan).

use std::collections::BTreeMap;

/// Which widget the Controls panel renders for a field.
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    Text,
    Number { min: Option<f64>, max: Option<f64>, step: Option<f64> },
    Range  { min: f64, max: f64, step: f64 },
    Toggle,
    Select { options: &'static [&'static str] },
    Radio  { options: &'static [&'static str] },
    Color,
    /// An `EventHandler` field: no widget, logs to the Actions panel instead.
    Action,
    /// Explicitly excluded (`#[control(skip)]`) or an untranslatable type.
    None,
}

/// One row of the props table / one control widget. The Rust analogue of
/// Storybook's `argTypes`, except produced statically by the compiler.
#[derive(Debug, Clone, PartialEq)]
pub struct ArgType {
    pub name: &'static str,
    /// Rendered source type, for the docs props table.
    pub ty: &'static str,
    /// Text of the field's `///` doc comments.
    pub docs: &'static str,
    pub control: Control,
    /// `false` for `Option<T>` and defaulted fields.
    pub required: bool,
}

/// A dynamic arg value, as it arrives from the Controls panel or the URL.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgValue {
    Text(String),
    Num(f64),
    Bool(bool),
    /// Enum variant selected by name.
    Variant(String),
    Null,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArgMap(pub BTreeMap<String, ArgValue>);

impl ArgMap {
    pub fn new() -> Self { Self(BTreeMap::new()) }
    pub fn set(mut self, k: &str, v: ArgValue) -> Self { self.0.insert(k.into(), v); self }
    pub fn get(&self, k: &str) -> Option<&ArgValue> { self.0.get(k) }
    /// Overlay: use the dynamic value if present *and* convertible,
    /// otherwise keep the story's typed default.
    pub fn get_or<T: FromArg>(&self, k: &str, default: T) -> T {
        self.0.get(k).and_then(T::from_arg).unwrap_or(default)
    }
}

/// Converting a dynamic value back into a typed field.
pub trait FromArg: Sized {
    fn from_arg(v: &ArgValue) -> Option<Self>;
}
/// Converting a typed field into a dynamic value, to seed the panel.
pub trait ToArg {
    fn to_arg(&self) -> ArgValue;
}

macro_rules! num_impl {
    ($($t:ty),*) => {$(
        impl FromArg for $t {
            fn from_arg(v: &ArgValue) -> Option<Self> {
                match v { ArgValue::Num(n) => Some(*n as $t),
                          ArgValue::Text(s) => s.parse().ok(), _ => None }
            }
        }
        impl ToArg for $t { fn to_arg(&self) -> ArgValue { ArgValue::Num(*self as f64) } }
    )*};
}
num_impl!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

impl FromArg for String {
    fn from_arg(v: &ArgValue) -> Option<Self> {
        match v {
            ArgValue::Text(s) | ArgValue::Variant(s) => Some(s.clone()),
            ArgValue::Num(n) => Some(n.to_string()),
            ArgValue::Bool(b) => Some(b.to_string()),
            ArgValue::Null => None,
        }
    }
}
impl ToArg for String { fn to_arg(&self) -> ArgValue { ArgValue::Text(self.clone()) } }

impl FromArg for bool {
    fn from_arg(v: &ArgValue) -> Option<Self> {
        match v { ArgValue::Bool(b) => Some(*b),
                  ArgValue::Text(s) => s.parse().ok(), _ => None }
    }
}
impl ToArg for bool { fn to_arg(&self) -> ArgValue { ArgValue::Bool(*self) } }

impl<T: FromArg> FromArg for Option<T> {
    fn from_arg(v: &ArgValue) -> Option<Self> {
        match v { ArgValue::Null => Some(None), other => Some(T::from_arg(other)) }
    }
}
impl<T: ToArg> ToArg for Option<T> {
    fn to_arg(&self) -> ArgValue {
        match self { Some(v) => v.to_arg(), None => ArgValue::Null }
    }
}

/// Implemented by `#[derive(ControlEnum)]`; supplies the Select options.
pub trait ControlEnum: Sized {
    const VARIANTS: &'static [&'static str];
    fn from_variant(s: &str) -> Option<Self>;
    fn variant_name(&self) -> &'static str;
}

/// Implemented by `#[derive(Controls)]` on a Props struct.
pub trait Controllable: Sized {
    fn arg_types() -> &'static [ArgType];
    /// Overlay dynamic args onto a typed base. This is P3's answer:
    /// an applier, not a deserializer.
    fn apply(&self, args: &ArgMap) -> Self;
    /// Seed the Controls panel from the story's declared props.
    fn to_args(&self) -> ArgMap;
}
