//! What `button.stories.rs` will look like once the macro exists. Written out
//! by hand here so the spike tests hot-reload, not codegen (settled in S2).

use dioxus::prelude::*;
use s3_core::{ArgMap, Controllable};

use crate::button::{Button, ButtonProps, ButtonVariant};
use crate::registry::StoryDef;

fn base() -> ButtonProps {
    ButtonProps {
        label: "Click me".into(),
        variant: ButtonVariant::Primary,
        scale: 1.0,
        disabled: false,
        tooltip: None,
        onclick: EventHandler::new(|_| crate::log_action("onclick")),
    }
}

fn render_with(args: &ArgMap, seed: fn() -> ButtonProps) -> Element {
    let props = seed().apply(args);
    rsx! { Button { ..props } }
}

macro_rules! story {
    ($ident:ident, $id:expr, $name:expr, $seed:expr) => {
        pub static $ident: StoryDef = StoryDef {
            id: $id,
            title: "Forms/Button",
            name: $name,
            arg_types: ButtonProps::arg_types,
            base_args: || $seed().to_args(),
            render: |args| render_with(args, $seed),
        };
    };
}

story!(PRIMARY, "forms-button--primary", "Primary", base);
story!(SECONDARY, "forms-button--secondary", "Secondary", || ButtonProps {
    variant: ButtonVariant::Secondary, label: "Cancel".into(), ..base()
});
story!(DANGER, "forms-button--danger", "Danger", || ButtonProps {
    variant: ButtonVariant::Danger, label: "Delete".into(),
    tooltip: Some("This cannot be undone".into()), ..base()
});
story!(DISABLED, "forms-button--disabled", "Disabled", || ButtonProps {
    disabled: true, ..base()
});

/// What the S2 build script will emit.
pub static STORIES: &[&StoryDef] = &[&PRIMARY, &SECONDARY, &DANGER, &DISABLED];
