//! Sidebar index behaviour: tree shape, collapse flattening, search ranking.

use std::collections::BTreeSet;

use dioxus_storybook_core::{ArgMap, Registry, Row, RowKind, StoryDef, TreeNode, flatten};

fn nil(_: &ArgMap) -> dioxus_core::Element {
    Ok(dioxus_core::VNode::placeholder())
}

static BUTTON_PRIMARY: StoryDef = StoryDef::new("Forms/Button", "Primary", nil);
static BUTTON_DANGER: StoryDef = StoryDef::new("Forms/Button", "Danger", nil);
static INPUT_TEXT: StoryDef = StoryDef::new("Forms/Input", "Text", nil);
static CARD: StoryDef = StoryDef::new("Layout/Card", "Default", nil);
static TOP: StoryDef = StoryDef::new("Welcome", "Intro", nil);

static ALL: &[&StoryDef] = &[
    &BUTTON_PRIMARY,
    &BUTTON_DANGER,
    &INPUT_TEXT,
    &CARD,
    &TOP,
];

fn registry() -> Registry {
    Registry::new(ALL)
}

#[test]
fn ids_are_kebab_title_and_name() {
    assert_eq!(BUTTON_PRIMARY.id(), "forms-button--primary");
    assert_eq!(TOP.id(), "welcome--intro");
}

#[test]
fn lookup_by_id_round_trips_every_story() {
    let r = registry();
    for story in r.stories() {
        let found = r.get(&story.id()).expect("registered story must resolve");
        assert_eq!(found.id(), story.id());
    }
    assert!(r.get("nope--missing").is_none());
}

#[test]
fn tree_nests_on_title_segments_in_first_seen_order() {
    let tree = registry().tree();

    let names: Vec<&str> = tree
        .iter()
        .map(|n| match n {
            TreeNode::Group(g) => g.name.as_str(),
            TreeNode::Story(s) => s.name,
        })
        .collect();
    // `Welcome` has no slash, so it is a group with one story under it.
    assert_eq!(names, ["Forms", "Layout", "Welcome"]);

    let TreeNode::Group(forms) = &tree[0] else {
        panic!("expected a group")
    };
    assert_eq!(forms.path, "Forms");
    let children: Vec<&str> = forms
        .children
        .iter()
        .map(|n| match n {
            TreeNode::Group(g) => g.name.as_str(),
            TreeNode::Story(s) => s.name,
        })
        .collect();
    assert_eq!(children, ["Button", "Input"]);

    let TreeNode::Group(button) = &forms.children[0] else {
        panic!("expected a group")
    };
    assert_eq!(button.path, "Forms/Button");
    assert_eq!(button.children.len(), 2, "both Button stories nest together");
}

#[test]
fn flatten_hides_children_of_collapsed_groups() {
    let tree = registry().tree();

    let all = flatten(&tree, &BTreeSet::new());
    let story_rows = all
        .iter()
        .filter(|r| matches!(r.kind, RowKind::Story { .. }))
        .count();
    assert_eq!(story_rows, 5, "every story is visible when nothing collapsed");

    let mut collapsed = BTreeSet::new();
    collapsed.insert("Forms".to_string());
    let rows = flatten(&tree, &collapsed);

    assert!(
        !rows.iter().any(|r| matches!(&r.kind, RowKind::Story { id, .. } if id.starts_with("forms-"))),
        "collapsing Forms hides its stories"
    );
    assert!(
        rows.iter().any(|r| matches!(&r.kind, RowKind::Group { path, expanded, .. }
            if path == "Forms" && !expanded)),
        "the collapsed group itself stays visible, marked collapsed"
    );
    assert!(
        rows.iter().any(|r| matches!(&r.kind, RowKind::Story { id, .. } if id == "layout-card--default")),
        "sibling groups are unaffected"
    );
}

#[test]
fn flatten_reports_depth_for_indentation() {
    let tree = registry().tree();
    let rows = flatten(&tree, &BTreeSet::new());
    let depths: Vec<(usize, String)> = rows
        .iter()
        .map(|Row { depth, kind }| {
            let label = match kind {
                RowKind::Group { name, .. } => name.clone(),
                RowKind::Story { name, .. } => (*name).to_string(),
            };
            (*depth, label)
        })
        .collect();
    assert_eq!(depths[0], (0, "Forms".into()));
    assert_eq!(depths[1], (1, "Button".into()));
    assert_eq!(depths[2], (2, "Primary".into()));
}

#[test]
fn empty_search_returns_index_order() {
    let hits = registry().search("   ");
    assert_eq!(hits.len(), 5);
    assert_eq!(hits[0].id(), "forms-button--primary");
}

#[test]
fn search_matches_subsequences_and_ranks_the_closest_first() {
    let r = registry();

    // Subsequence, not substring: b-t-n threads through "Forms/Button".
    let hits = r.search("btn");
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|s| s.title() == "Forms/Button"));

    let hits = r.search("zqx");
    assert!(hits.is_empty(), "a non-subsequence matches nothing");

    let hits = r.search("button");
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|s| s.title() == "Forms/Button"));

    let hits = r.search("card");
    assert_eq!(hits[0].id(), "layout-card--default");

    let hits = r.search("prim");
    assert_eq!(hits[0].id(), "forms-button--primary");
}
