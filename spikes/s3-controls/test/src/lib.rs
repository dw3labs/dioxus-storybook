//! S3 spike: does `#[derive(Controls)]` survive contact with a REAL Dioxus
//! Props struct — `Element`, `EventHandler<MouseEvent>`, the `Props` derive
//! and all — and produce a correct props table plus a working applier?

use dioxus::prelude::*;
use s3_core::{ArgMap, ArgValue, Control, Controllable, ControlEnum};
use s3_macro::{ControlEnum, Controls};

#[derive(Clone, Copy, PartialEq, Debug, ControlEnum)]
pub enum ButtonVariant { Primary, Secondary, Ghost, Danger }

/// A button.
#[derive(Props, Clone, PartialEq, Controls)]
pub struct ButtonProps {
    /// Text shown inside the button.
    pub label: String,

    /// Visual emphasis of the button.
    pub variant: ButtonVariant,

    /// Relative size multiplier.
    #[control(range(min = 0.5, max = 4.0, step = 0.25))]
    pub scale: f32,

    /// Whether the button rejects interaction.
    pub disabled: bool,

    /// Optional tooltip shown on hover.
    pub tooltip: Option<String>,

    /// Brand tint, as a CSS colour.
    #[control(color)]
    pub tint: String,

    /// Slot content — never a control.
    pub children: Element,

    /// Fired on click. Logged to the Actions panel.
    pub onclick: EventHandler<MouseEvent>,
}

/// Proves the generated props actually drive a real component.
#[component]
pub fn Button(props: ButtonProps) -> Element {
    rsx! {
        button {
            disabled: props.disabled,
            style: "transform: scale({props.scale}); color: {props.tint}",
            onclick: move |e| props.onclick.call(e),
            "{props.label}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arg(name: &str) -> &'static s3_core::ArgType {
        ButtonProps::arg_types().iter().find(|a| a.name == name)
            .unwrap_or_else(|| panic!("no argType for {name}"))
    }

    #[test]
    fn every_field_is_described() {
        assert_eq!(ButtonProps::arg_types().len(), 8);
    }

    #[test]
    fn doc_comments_reach_the_macro() {
        assert_eq!(arg("label").docs, "Text shown inside the button.");
        assert_eq!(arg("tooltip").docs, "Optional tooltip shown on hover.");
        assert_eq!(arg("onclick").docs, "Fired on click. Logged to the Actions panel.");
    }

    #[test]
    fn controls_are_inferred_from_types() {
        assert_eq!(arg("label").control, Control::Text);
        assert_eq!(arg("disabled").control, Control::Toggle);
        assert!(matches!(arg("tooltip").control, Control::Text));
    }

    #[test]
    fn enums_become_selects_with_real_variants() {
        assert_eq!(
            arg("variant").control,
            Control::Select { options: &["Primary", "Secondary", "Ghost", "Danger"] }
        );
    }

    #[test]
    fn attribute_overrides_win_over_inference() {
        assert_eq!(arg("scale").control, Control::Range { min: 0.5, max: 4.0, step: 0.25 });
        assert_eq!(arg("tint").control, Control::Color); // would infer Text
    }

    #[test]
    fn dioxus_types_are_handled() {
        assert_eq!(arg("children").control, Control::None);   // Element
        assert_eq!(arg("onclick").control, Control::Action);  // EventHandler
    }

    #[test]
    fn optionality_is_recorded() {
        assert!(arg("label").required);
        assert!(!arg("tooltip").required, "Option<T> must be optional");
    }

    #[test]
    fn rendered_types_are_kept_for_the_props_table() {
        assert_eq!(arg("tooltip").ty, "Option<String>");
        assert_eq!(arg("onclick").ty, "EventHandler<MouseEvent>");
    }

    /// `rsx!` and `EventHandler::new` both require an active Dioxus runtime,
    /// so the applier tests run inside a real VirtualDom rather than bare.
    fn with_runtime<T>(f: impl FnOnce() -> T) -> T {
        #[component]
        fn Empty() -> Element { rsx! {} }
        let mut dom = VirtualDom::new(Empty);
        dom.rebuild_in_place();
        dom.in_scope(ScopeId::ROOT, f)
    }

    fn base() -> ButtonProps {
        ButtonProps {
            label: "Click me".into(),
            variant: ButtonVariant::Primary,
            scale: 1.0,
            disabled: false,
            tooltip: None,
            tint: "#C4451B".into(),
            children: rsx! { "" },
            onclick: EventHandler::new(|_| {}),
        }
    }

    #[test]
    fn apply_overlays_only_supplied_args() {
        with_runtime(|| {
            let args = ArgMap::new()
                .set("label", ArgValue::Text("Save".into()))
                .set("disabled", ArgValue::Bool(true));
            let next = base().apply(&args);

            assert_eq!(next.label, "Save");            // overridden
            assert!(next.disabled);                    // overridden
            assert_eq!(next.scale, 1.0);               // untouched
            assert_eq!(next.tint, "#C4451B");          // untouched
            assert_eq!(next.variant, ButtonVariant::Primary);
        });
    }

    #[test]
    fn enum_args_round_trip_by_variant_name() {
        with_runtime(|| {
            let args = ArgMap::new().set("variant", ArgValue::Variant("Danger".into()));
            assert_eq!(base().apply(&args).variant, ButtonVariant::Danger);
        });
    }

    #[test]
    fn option_fields_accept_values_and_null() {
        with_runtime(|| {
            let set = ArgMap::new().set("tooltip", ArgValue::Text("Saves your work".into()));
            assert_eq!(base().apply(&set).tooltip.as_deref(), Some("Saves your work"));

            let mut with = base();
            with.tooltip = Some("x".into());
            let cleared = with.apply(&ArgMap::new().set("tooltip", ArgValue::Null));
            assert_eq!(cleared.tooltip, None, "Null must clear an Option");
        });
    }

    #[test]
    fn garbage_args_fall_back_to_the_typed_default() {
        with_runtime(|| {
            // A bad value from the URL must not blow up the preview.
            let args = ArgMap::new()
                .set("scale", ArgValue::Text("not-a-number".into()))
                .set("variant", ArgValue::Variant("Nonexistent".into()));
            let next = base().apply(&args);
            assert_eq!(next.scale, 1.0);
            assert_eq!(next.variant, ButtonVariant::Primary);
        });
    }

    #[test]
    fn unknown_arg_keys_are_ignored() {
        with_runtime(|| {
            let next = base().apply(&ArgMap::new().set("nope", ArgValue::Bool(true)));
            assert_eq!(next.label, "Click me");
        });
    }

    #[test]
    fn to_args_seeds_the_panel_and_excludes_non_controls() {
        with_runtime(|| {
            let m = base().to_args();
            assert_eq!(m.get("label"), Some(&ArgValue::Text("Click me".into())));
            assert_eq!(m.get("variant"), Some(&ArgValue::Variant("Primary".into())));
            assert_eq!(m.get("tooltip"), Some(&ArgValue::Null));
            assert!(m.get("children").is_none(), "Element must not be seeded");
            assert!(m.get("onclick").is_none(), "EventHandler must not be seeded");
        });
    }

    #[test]
    fn apply_is_idempotent_through_a_round_trip() {
        with_runtime(|| {
            let b = base();
            let round = b.apply(&b.to_args());
            assert_eq!(round.label, b.label);
            assert_eq!(round.variant, b.variant);
            assert_eq!(round.scale, b.scale);
            assert_eq!(round.tooltip, b.tooltip);
        });
    }
}
