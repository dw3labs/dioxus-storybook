//! Viewports: the size of the canvas a story is rendered into.
//!
//! Checking that a component survives a 360px phone is the one thing a
//! workbench can do that a page in your app cannot, and from M3 it is nearly
//! free: the preview is a **frame whose width the shell owns**. Nothing has to
//! be told to re-render, no media-query shim is needed, and the story sees a
//! real viewport because it *is* in one.
//!
//! # Which half of it is a parameter and which is a global
//!
//! Storybook splits this in two, and the split maps onto the two mechanisms
//! this crate already has:
//!
//! | | what | where it lives |
//! |---|---|---|
//! | the list of sizes on offer | a static declaration | [`Project::viewports`] |
//! | the size a story opens at | static, per level | the `viewport` **parameter** |
//! | the size you picked | run-time, book-wide | the `viewport` **global** |
//!
//! The available list is on the [`Project`] rather than in [`Parameters`] for a
//! reason worth stating: a [`ParamValue`] is a string, a number or a bool, and a
//! viewport is a record. Widening `ParamValue` to hold one would be the fourth
//! value vocabulary this crate has spent three milestones not growing. The
//! *choice*, which is the half that genuinely varies per story, is a parameter
//! and needs nothing new.
//!
//! The selection is a global — it is not one the project declares, but it is
//! carried in the same [`ArgMap`], which means it is in the URL
//! (`&globals=viewport:tablet`), on the wire in `SetGlobals`, and visible to
//! decorators through [`StoryContext::globals`], with no new code anywhere.
//! Two names are reserved in that map for it: [`VIEWPORT_GLOBAL`] and
//! [`ROTATED_GLOBAL`].
//!
//! # Resolution
//!
//! [`resolve`] is the whole decision, in one place, called by both halves — the
//! manager to size the frame, the preview to fill in
//! [`StoryContext::viewport`]. Like the shell's landing-story rule before it,
//! sharing one function is what stops the two answers drifting apart.
//!
//! [`Parameters`]: crate::Parameters
//! [`ParamValue`]: crate::ParamValue
//! [`Project`]: crate::Project
//! [`Project::viewports`]: crate::Project::viewports
//! [`StoryContext::globals`]: crate::StoryContext::globals
//! [`StoryContext::viewport`]: crate::StoryContext::viewport

use crate::{ArgMap, ArgValue, ResolvedParameters};

/// The reserved global — and parameter — naming the selected viewport.
pub const VIEWPORT_GLOBAL: &str = "viewport";

/// The reserved global holding whether the viewport is turned on its side.
pub const ROTATED_GLOBAL: &str = "viewport-rotated";

/// The reserved viewport name meaning "no fixed size; fill the pane".
///
/// It is a real, selectable value rather than the absence of one, because
/// absence already means something else: "whatever this story asked for".
/// Without it a story carrying `parameters { viewport: "mobile" }` could never
/// be looked at full-width.
pub const RESPONSIVE: &str = "responsive";

/// One selectable canvas size.
///
/// Fields are private and built with a `const` constructor, so adding one later
/// is a new `with_*` method rather than a break for every caller.
///
/// ```
/// # use dioxus_storybook_core::Viewport;
/// static SIZES: &[Viewport] = &[
///     Viewport::new("phone", "Phone", 390, 844),
///     Viewport::new("desk", "Desk", 1440, 900),
/// ];
/// assert_eq!(SIZES[0].width(), 390);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    name: &'static str,
    title: &'static str,
    width: u32,
    height: u32,
}

impl Viewport {
    /// A viewport with a URL-safe `name`, a toolbar `title`, and a size in CSS
    /// pixels.
    pub const fn new(name: &'static str, title: &'static str, width: u32, height: u32) -> Self {
        Self { name, title, width, height }
    }

    /// The key this viewport is selected by, in the URL and on the wire.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The label the picker shows.
    pub const fn title(&self) -> &'static str {
        self.title
    }

    /// Width in CSS pixels, unrotated.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in CSS pixels, unrotated.
    pub const fn height(&self) -> u32 {
        self.height
    }
}

/// The viewports every storybook offers unless the project says otherwise.
///
/// Deliberately short. A picker is a list you read top to bottom every time you
/// use it, so the useful thing is one entry per *decision* — phone, big phone,
/// tablet, laptop, monitor — not one per device that has ever shipped.
pub static DEFAULT_VIEWPORTS: &[Viewport] = &[
    Viewport::new("mobile", "Mobile", 360, 640),
    Viewport::new("mobile-large", "Mobile L", 414, 896),
    Viewport::new("tablet", "Tablet", 834, 1112),
    Viewport::new("laptop", "Laptop", 1280, 800),
    Viewport::new("desktop", "Desktop", 1680, 1050),
];

/// A resolved viewport: which one, and which way up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportSelection {
    viewport: &'static Viewport,
    rotated: bool,
}

impl ViewportSelection {
    /// The viewport that was chosen.
    pub const fn viewport(&self) -> &'static Viewport {
        self.viewport
    }

    /// Its name, for round-tripping back into a picker.
    pub const fn name(&self) -> &'static str {
        self.viewport.name
    }

    /// Whether the canvas is turned on its side.
    pub const fn rotated(&self) -> bool {
        self.rotated
    }

    /// The width to give the canvas, rotation included.
    pub const fn width(&self) -> u32 {
        if self.rotated { self.viewport.height } else { self.viewport.width }
    }

    /// The height to give the canvas, rotation included.
    pub const fn height(&self) -> u32 {
        if self.rotated { self.viewport.width } else { self.viewport.height }
    }
}

/// Look one viewport up by name.
pub fn find(available: &'static [Viewport], name: &str) -> Option<&'static Viewport> {
    available.iter().find(|v| v.name == name)
}

/// The size the canvas should be: the selection if there is one, otherwise
/// whatever the story's parameters asked for, otherwise responsive.
///
/// `selected` is the *selection* map — the globals a user has actually moved,
/// not the resolved set — because "nothing selected" is a distinct answer from
/// "responsive selected". See [`RESPONSIVE`].
///
/// Nothing here is an error. A link naming a viewport this build does not have,
/// a parameter with a typo in it, and a project with no viewports at all all
/// resolve to responsive, because every one of them is more usefully a canvas
/// than a crash.
///
/// ```
/// # use dioxus_storybook_core::{ArgMap, ArgValue, ResolvedParameters, Viewport, viewport};
/// static SIZES: &[Viewport] = &[Viewport::new("phone", "Phone", 390, 844)];
///
/// let picked = ArgMap::new().with("viewport", ArgValue::Variant("phone".into()));
/// let resolved = viewport::resolve(SIZES, &picked, ResolvedParameters::default()).unwrap();
/// assert_eq!((resolved.width(), resolved.height()), (390, 844));
///
/// // Nothing selected, nothing asked for.
/// assert!(viewport::resolve(SIZES, &ArgMap::new(), ResolvedParameters::default()).is_none());
/// ```
pub fn resolve(
    available: &'static [Viewport],
    selected: &ArgMap,
    parameters: ResolvedParameters,
) -> Option<ViewportSelection> {
    if available.is_empty() {
        return None;
    }
    // The URL codec is untyped, so a pasted `viewport:tablet` arrives as
    // `Text` and a picked one as `Variant`. `as_text` is the coercion; there is
    // nothing else to get right here.
    let chosen = match selected.get(VIEWPORT_GLOBAL) {
        Some(value) => value.as_text(),
        None => parameters.str(VIEWPORT_GLOBAL)?.to_string(),
    };
    if chosen.is_empty() || chosen == RESPONSIVE {
        return None;
    }
    let viewport = find(available, &chosen)?;
    let rotated = selected
        .get(ROTATED_GLOBAL)
        .and_then(ArgValue::as_bool)
        .unwrap_or(false);
    Some(ViewportSelection { viewport, rotated })
}

/// Whether an explicit `responsive` selection is needed to *mean* responsive.
///
/// Picking "Responsive" normally means removing the entry, which keeps it out
/// of the link and lets an empty selection map keep meaning "nothing has been
/// changed". It cannot mean that when a story's parameters name a viewport:
/// removing the entry there would snap straight back to the story's choice. So
/// the picker asks this, and stores the word only when the word is doing work.
pub fn responsive_needs_saying(
    available: &'static [Viewport],
    selected: &ArgMap,
    parameters: ResolvedParameters,
) -> bool {
    let mut without = selected.clone();
    without.remove(VIEWPORT_GLOBAL);
    resolve(available, &without, parameters).is_some()
}
