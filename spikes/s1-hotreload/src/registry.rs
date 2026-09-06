use dioxus::prelude::*;
use s3_core::{ArgMap, ArgType};

/// The shape the real `dx-story-core::StoryDef` will have.
///
/// S3 established that `rsx!` and `EventHandler::new` need an active Dioxus
/// scope, so `base_args` and `render` are fn pointers invoked from inside the
/// preview component rather than evaluated up front.
pub struct StoryDef {
    pub id: &'static str,
    pub title: &'static str,
    pub name: &'static str,
    pub arg_types: fn() -> &'static [ArgType],
    pub base_args: fn() -> ArgMap,
    pub render: fn(&ArgMap) -> Element,
}
