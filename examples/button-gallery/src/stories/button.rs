//! Stories for `Button`.
//!
//! This file is never declared as a module by hand — `build.rs` finds it and
//! the generated registry mounts it.

use dioxus::prelude::*;
use dioxus_storybook::prelude::*;

use crate::button::{Button, ButtonProps, ButtonVariant};

story_meta! {
    title: "Forms/Button",
    component: Button,
    tags: ["autodocs"],
}

/// The props every story here starts from.
fn base() -> ButtonProps {
    ButtonProps {
        label: "Click me".into(),
        variant: ButtonVariant::Primary,
        scale: 1.0,
        disabled: false,
        tooltip: None,
        onclick: EventHandler::new(|_| {}),
    }
}

#[story]
fn primary() -> ButtonProps {
    base()
}

#[story]
fn secondary() -> ButtonProps {
    ButtonProps {
        variant: ButtonVariant::Secondary,
        label: "Cancel".into(),
        ..base()
    }
}

#[story]
fn danger() -> ButtonProps {
    ButtonProps {
        variant: ButtonVariant::Danger,
        label: "Delete".into(),
        tooltip: Some("This cannot be undone".into()),
        ..base()
    }
}

#[story]
fn disabled() -> ButtonProps {
    ButtonProps {
        disabled: true,
        ..base()
    }
}

#[story(name = "Oversized")]
fn oversized() -> ButtonProps {
    ButtonProps {
        scale: 2.0,
        label: "Big".into(),
        ..base()
    }
}

/// The escape hatch: return `Element` and compose whatever you like. Such a
/// story opts out of auto-generated controls, because there is no props type to
/// introspect.
#[story(name = "Button Row")]
fn button_row() -> Element {
    rsx! {
        div { style: "display:flex; gap:10px; align-items:center;",
            Button { ..base() }
            Button { variant: ButtonVariant::Secondary, label: "Cancel".into(), ..base() }
            Button { variant: ButtonVariant::Ghost, label: "Learn more".into(), ..base() }
        }
    }
}
