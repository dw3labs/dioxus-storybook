//! Stories for `Badge`, under a different sidebar group.

use dioxus_storybook::prelude::*;

use crate::badge::{Badge, BadgeProps};

story_meta! {
    title: "Data Display/Badge",
    component: Badge,
}

// No `description:` here on purpose: the docs page falls back to the `///` on
// `BadgeProps`, so a component is documented without anyone writing a word of
// storybook-specific prose.

fn base() -> BadgeProps {
    BadgeProps {
        text: "New".into(),
        color: "#C4451B".into(),
        outline: false,
    }
}

#[story]
fn solid() -> BadgeProps {
    base()
}

#[story]
fn outline() -> BadgeProps {
    BadgeProps {
        outline: true,
        ..base()
    }
}

#[story]
fn long_label() -> BadgeProps {
    BadgeProps {
        text: "Deprecated since 0.4".into(),
        color: "#787F88".into(),
        ..base()
    }
}
