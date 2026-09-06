//! Actions: watching a component call the handlers it was given.
//!
//! Storybook's actions addon works by *replacing* the handler props of a story
//! with instrumented ones. We do the same, except the replacement is generated
//! by `#[derive(Controls)]` — see [`Controllable::wire_actions`] — so it is
//! type-checked rather than reflective.
//!
//! Two ambient values make that work without threading arguments through every
//! story signature. Both are Dioxus contexts provided by the preview, and both
//! degrade to a no-op when absent, so a story function is still callable from a
//! plain test:
//!
//! - [`ActionSink`] — where a wired handler reports its calls.
//! - [`ArgsHandle`] — how a story writes an arg back to the controls panel.
//!
//! [`Controllable::wire_actions`]: crate::Controllable::wire_actions

use std::rc::Rc;

use crate::ArgValue;

/// Where instrumented event handlers report their calls.
///
/// Cheap to clone. A default-constructed sink is *disabled*: it accepts calls
/// and drops them, which is what makes a story function usable outside a
/// running storybook.
#[derive(Clone, Default)]
pub struct ActionSink {
    emit: Option<Rc<dyn Fn(&str, String)>>,
}

impl ActionSink {
    /// A sink that forwards every call to `emit`, as `(prop name, payload)`.
    pub fn new(emit: impl Fn(&str, String) + 'static) -> Self {
        Self {
            emit: Some(Rc::new(emit)),
        }
    }

    /// A sink that discards everything. The default.
    pub fn disabled() -> Self {
        Self::default()
    }

    /// `true` if calls are actually going somewhere.
    pub fn is_enabled(&self) -> bool {
        self.emit.is_some()
    }

    /// Report one handler call.
    pub fn log(&self, name: &str, payload: String) {
        if let Some(emit) = &self.emit {
            emit(name, payload);
        }
    }

    /// The sink the preview provided for the story being rendered, or a
    /// disabled one when called outside a preview.
    pub fn ambient() -> Self {
        ambient_context().unwrap_or_default()
    }
}

impl core::fmt::Debug for ActionSink {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ActionSink")
            .field("enabled", &self.is_enabled())
            .finish()
    }
}

/// Two sinks are equal when they forward to the same place.
///
/// Needed because a sink is handed to `wire_actions` from inside a render, and
/// anything reachable from props gets compared.
impl PartialEq for ActionSink {
    fn eq(&self, other: &Self) -> bool {
        match (&self.emit, &other.emit) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

/// A story's write-back channel into the controls panel.
///
/// Rendering a *controlled* input — one whose value comes from a prop and whose
/// `oninput` must change that prop — needs the story to push a new arg value
/// outward. This is that push. The manager merges it and broadcasts the result,
/// so the panel, the URL and the preview stay one source of truth rather than
/// three.
///
/// Obtain one with [`use_args`]. Like [`ActionSink`], a default handle is
/// disabled and drops writes.
#[derive(Clone, Default)]
pub struct ArgsHandle {
    write: Option<Rc<dyn Fn(String, ArgValue)>>,
}

impl ArgsHandle {
    /// A handle that forwards each write to `write`.
    pub fn new(write: impl Fn(String, ArgValue) + 'static) -> Self {
        Self {
            write: Some(Rc::new(write)),
        }
    }

    /// A handle that discards writes. The default.
    pub fn disabled() -> Self {
        Self::default()
    }

    /// `true` if writes are actually going somewhere.
    pub fn is_enabled(&self) -> bool {
        self.write.is_some()
    }

    /// Set one arg, as if the controls panel had been edited.
    pub fn set(&self, name: impl Into<String>, value: ArgValue) {
        if let Some(write) = &self.write {
            write(name.into(), value);
        }
    }
}

impl core::fmt::Debug for ArgsHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ArgsHandle")
            .field("enabled", &self.is_enabled())
            .finish()
    }
}

impl PartialEq for ArgsHandle {
    fn eq(&self, other: &Self) -> bool {
        match (&self.write, &other.write) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

/// The write-back handle for the story currently being rendered.
///
/// Not a hook despite the name — it reads a Dioxus context on every call, so it
/// is safe inside event handlers and closures as well as in a story body.
/// Outside a preview, and outside a Dioxus runtime entirely, it returns a
/// disabled handle rather than panicking.
///
/// ```ignore
/// #[story]
/// fn editable(args: &ArgMap) -> Element {
///     let args_out = use_args();
///     let text = args.get("label").map(ArgValue::as_text).unwrap_or_default();
///     rsx! {
///         input {
///             value: "{text}",
///             oninput: move |e| args_out.set("label", ArgValue::Text(e.value())),
///         }
///     }
/// }
/// ```
pub fn use_args() -> ArgsHandle {
    ambient_context().unwrap_or_default()
}

/// Read a context without requiring that a Dioxus runtime exists at all.
///
/// `try_consume_context` is only forgiving about a *missing context*: with no
/// runtime on the stack it panics. Both values here are read from code the
/// macros generate into every story body, so "there is no runtime" has to be an
/// answer rather than a crash — otherwise a plain unit test that touches a
/// story gets a panic from a diagnostic path rather than from its own mistake.
fn ambient_context<T: Clone + 'static>() -> Option<T> {
    dioxus_core::Runtime::try_current()?;
    dioxus_core::try_consume_context::<T>()
}

/// Rendering an action payload without demanding `Debug` from the user.
///
/// Generated code cannot know whether an `EventHandler<T>`'s payload is
/// printable, and requiring `T: Debug` would put a bound on other people's prop
/// types just because they wanted an actions panel. So the choice is made by
/// method resolution instead: the inherent method below applies only when
/// `T: Debug`, and the trait method is what is left when it does not.
///
/// Not part of the public API and exempt from semver.
#[doc(hidden)]
pub mod describe {
    use core::fmt::Debug;

    /// Wrapper that carries the payload into method resolution.
    pub struct Probe<T>(pub T);

    impl<T: Debug> Probe<&T> {
        /// The printable case: inherent methods are tried first, so this wins
        /// whenever the bound is satisfied.
        pub fn dxsb_describe(&self) -> String {
            format!("{:?}", self.0)
        }
    }

    /// The fallback, reached only when the inherent method does not apply.
    pub trait DescribeFallback {
        /// Describe a payload that does not implement `Debug`.
        fn dxsb_describe(&self) -> String;
    }

    impl<T> DescribeFallback for Probe<&T> {
        fn dxsb_describe(&self) -> String {
            String::from("…")
        }
    }
}
