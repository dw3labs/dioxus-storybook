//! Stories for `Field` — the M2 features that a static component cannot show.

use dioxus::prelude::*;
use dioxus_storybook::prelude::*;

use crate::field::{Field, FieldProps, FieldStatus};

story_meta! {
    title: "Forms/Field",
    component: Field,
}

fn base() -> FieldProps {
    FieldProps {
        label: "Project name".into(),
        value: String::new(),
        placeholder: "acme-web".into(),
        status: FieldStatus::Neutral,
        disabled: false,
        // Left empty on purpose. Nothing here logs anything — the actions panel
        // sees these calls because `#[derive(Controls)]` wraps every
        // `EventHandler` prop before the component is handed its props.
        oninput: EventHandler::new(|_| {}),
        onblur: EventHandler::new(|_| {}),
    }
}

/// The plain form. Type into it and the actions panel fills up with `oninput`
/// calls — but watch the `value` row in the controls panel: it stays empty. The
/// browser is keeping your keystrokes in its own DOM state; the *prop* never
/// moved, because nothing is updating it. [`Controlled`](CONTROLLED) is the
/// same field with that gap closed.
#[story]
fn empty() -> FieldProps {
    base()
}

#[story]
fn filled() -> FieldProps {
    FieldProps {
        value: "acme-web".into(),
        status: FieldStatus::Valid,
        ..base()
    }
}

#[story]
fn invalid() -> FieldProps {
    FieldProps {
        label: "Project name".into(),
        value: "Acme Web!".into(),
        status: FieldStatus::Invalid,
        ..base()
    }
}

#[story]
fn disabled() -> FieldProps {
    FieldProps {
        value: "locked".into(),
        disabled: true,
        ..base()
    }
}

/// A **controlled** field that actually accepts typing.
///
/// Still the props form, so it keeps its full controls table — a story body runs
/// inside the preview's scope and may call [`use_args`] just as well as an
/// `Element`-form story can. The `oninput` handler writes the new text back as
/// the `value` arg; the manager merges it, re-broadcasts, and the applier
/// overlays it on these props. Type into the field and watch the `value` row and
/// the address bar move with you.
///
/// The handler is *also* wrapped for the actions panel, so every keystroke shows
/// up in both places at once.
#[story(name = "Controlled")]
fn controlled() -> FieldProps {
    let args_out = use_args();
    FieldProps {
        label: "Controlled (writes its own args back)".into(),
        value: "type here".into(),
        placeholder: "start typing".into(),
        oninput: EventHandler::new(move |next: String| {
            args_out.set("value", ArgValue::Text(next));
        }),
        ..base()
    }
}
