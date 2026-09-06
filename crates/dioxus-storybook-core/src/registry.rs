//! The story index: lookup, the sidebar tree, and search.
//!
//! The registry itself is a plain `&'static [&'static StoryDef]` produced at
//! build time by [`dioxus-storybook-build`]. That is not an aesthetic choice:
//! `linkme` does not compile for `wasm32`, and `inventory` silently drops any
//! story in an unreferenced codegen unit — passing at `codegen-units = 1` and
//! failing at 16 and 256, which means stories vanish in dev builds and reappear
//! in release. Build-script codegen emits explicit references that no linker
//! decision can drop. See `log/0002-m0-spikes.md` before revisiting.
//!
//! [`dioxus-storybook-build`]: https://crates.io/crates/dioxus-storybook-build

use std::collections::BTreeSet;

use crate::StoryDef;

/// A handle onto the generated story table.
///
/// `Copy`, so it can be passed as a Dioxus prop without ceremony.
#[derive(Debug, Clone, Copy)]
pub struct Registry {
    stories: &'static [&'static StoryDef],
}

impl PartialEq for Registry {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.stories, other.stories)
    }
}

impl Registry {
    /// Wrap the generated table, normally the `STORIES` static emitted into
    /// `OUT_DIR` by the build script.
    pub const fn new(stories: &'static [&'static StoryDef]) -> Self {
        Self { stories }
    }

    /// Every story, in index order (file path, then declaration order).
    pub const fn stories(&self) -> &'static [&'static StoryDef] {
        self.stories
    }

    /// How many stories are registered.
    pub const fn len(&self) -> usize {
        self.stories.len()
    }

    /// `true` if no stories are registered — usually a build-script wiring bug.
    pub const fn is_empty(&self) -> bool {
        self.stories.is_empty()
    }

    /// Find a story by its [`StoryDef::id`].
    pub fn get(&self, id: &str) -> Option<&'static StoryDef> {
        self.stories.iter().copied().find(|s| s.id() == id)
    }

    /// The first story in index order, used as the landing story.
    pub fn first(&self) -> Option<&'static StoryDef> {
        self.stories.first().copied()
    }

    /// Build the sidebar tree by splitting each story's `title` on `/`.
    ///
    /// Groups and stories keep first-seen order, so the sidebar mirrors the
    /// order the build script indexed files in rather than re-sorting behind
    /// the author's back.
    pub fn tree(&self) -> Vec<TreeNode> {
        self.tree_filtered(|_| true)
    }

    /// Like [`Registry::tree`], but only including stories matching `keep`.
    pub fn tree_filtered(&self, keep: impl Fn(&'static StoryDef) -> bool) -> Vec<TreeNode> {
        let mut roots: Vec<TreeNode> = Vec::new();
        for story in self.stories.iter().copied().filter(|s| keep(s)) {
            let mut level = &mut roots;
            let mut path = String::new();
            for segment in story.title().split('/').filter(|s| !s.is_empty()) {
                if !path.is_empty() {
                    path.push('/');
                }
                path.push_str(segment);

                let existing = level.iter().position(|n| match n {
                    TreeNode::Group(g) => g.path == path,
                    TreeNode::Story(_) => false,
                });
                let index = match existing {
                    Some(i) => i,
                    None => {
                        level.push(TreeNode::Group(Group {
                            name: segment.to_string(),
                            path: path.clone(),
                            children: Vec::new(),
                        }));
                        level.len() - 1
                    }
                };
                let TreeNode::Group(group) = &mut level[index] else {
                    unreachable!("index came from a Group match")
                };
                level = &mut group.children;
            }
            level.push(TreeNode::Story(StoryRef {
                id: story.id(),
                name: story.name(),
                def: story,
            }));
        }
        roots
    }

    /// Stories whose title or name fuzzy-matches `query`, best match first.
    ///
    /// An empty query matches everything, in index order.
    pub fn search(&self, query: &str) -> Vec<&'static StoryDef> {
        if query.trim().is_empty() {
            return self.stories.to_vec();
        }
        let mut hits: Vec<(i32, usize, &'static StoryDef)> = self
            .stories
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(i, s)| {
                let haystack = format!("{}/{}", s.title(), s.name());
                fuzzy_score(query, &haystack).map(|score| (score, i, s))
            })
            .collect();
        // Higher score first; ties broken by index order so results are stable.
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        hits.into_iter().map(|(_, _, s)| s).collect()
    }
}

/// A node in the sidebar tree.
#[derive(Debug, Clone, PartialEq)]
pub enum TreeNode {
    /// A title segment, e.g. `Forms` or `Forms/Button`.
    Group(Group),
    /// A leaf story.
    Story(StoryRef),
}

/// A title segment and everything nested under it.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// The segment's own name, e.g. `Button`.
    pub name: String,
    /// The full path from the root, e.g. `Forms/Button`. Unique; used as the
    /// collapse key.
    pub path: String,
    /// Nested groups and stories, in first-seen order.
    pub children: Vec<TreeNode>,
}

/// A story as it appears in the sidebar.
#[derive(Debug, Clone)]
pub struct StoryRef {
    /// The story's [`StoryDef::id`].
    pub id: String,
    /// The story's display name.
    pub name: &'static str,
    /// The story itself.
    pub def: &'static StoryDef,
}

impl PartialEq for StoryRef {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// One visible row of the sidebar, once collapse state is applied.
///
/// Keyboard navigation walks this flat list, which is why it lives here and is
/// unit-testable rather than being derived inside the UI.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Nesting depth, starting at 0.
    pub depth: usize,
    /// What this row shows.
    pub kind: RowKind,
}

/// The two kinds of sidebar row.
#[derive(Debug, Clone, PartialEq)]
pub enum RowKind {
    /// A collapsible group.
    Group {
        /// The segment name.
        name: String,
        /// The full path, used as the collapse key.
        path: String,
        /// Whether the group is currently expanded.
        expanded: bool,
    },
    /// A selectable story.
    Story {
        /// The story id.
        id: String,
        /// The display name.
        name: &'static str,
    },
}

/// Flatten a tree into the rows the sidebar actually draws, hiding the
/// children of any group whose path is in `collapsed`.
pub fn flatten(nodes: &[TreeNode], collapsed: &BTreeSet<String>) -> Vec<Row> {
    fn walk(nodes: &[TreeNode], collapsed: &BTreeSet<String>, depth: usize, out: &mut Vec<Row>) {
        for node in nodes {
            match node {
                TreeNode::Group(g) => {
                    let expanded = !collapsed.contains(&g.path);
                    out.push(Row {
                        depth,
                        kind: RowKind::Group {
                            name: g.name.clone(),
                            path: g.path.clone(),
                            expanded,
                        },
                    });
                    if expanded {
                        walk(&g.children, collapsed, depth + 1, out);
                    }
                }
                TreeNode::Story(s) => out.push(Row {
                    depth,
                    kind: RowKind::Story {
                        id: s.id.clone(),
                        name: s.name,
                    },
                }),
            }
        }
    }
    let mut out = Vec::new();
    walk(nodes, collapsed, 0, &mut out);
    out
}

/// Every group path in a tree, for "expand all" / initial state.
pub fn group_paths(nodes: &[TreeNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(nodes: &[TreeNode], out: &mut Vec<String>) {
        for node in nodes {
            if let TreeNode::Group(g) = node {
                out.push(g.path.clone());
                walk(&g.children, out);
            }
        }
    }
    walk(nodes, &mut out);
    out
}

/// Score `query` against `haystack` as a case-insensitive subsequence match.
///
/// `None` means no match. Higher is better: consecutive characters and matches
/// at a word boundary score more, so `bt` ranks `Button` above `Breadcrumb Text`.
///
/// ```
/// # use dioxus_storybook_core::fuzzy_score;
/// assert!(fuzzy_score("btn", "Forms/Button").is_some());
/// assert!(fuzzy_score("zzz", "Forms/Button").is_none());
/// assert!(fuzzy_score("but", "Button") > fuzzy_score("but", "Breadcrumb Utility"));
/// ```
pub fn fuzzy_score(query: &str, haystack: &str) -> Option<i32> {
    let needle: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if needle.is_empty() {
        return Some(0);
    }
    let hay: Vec<char> = haystack.chars().collect();

    let mut score = 0i32;
    let mut n = 0usize;
    let mut last_hit: Option<usize> = None;

    for (i, hc) in hay.iter().enumerate() {
        if n == needle.len() {
            break;
        }
        if hc.to_ascii_lowercase() != needle[n] {
            continue;
        }
        score += 1;
        if last_hit == Some(i.wrapping_sub(1)) {
            score += 5; // consecutive run
        }
        let boundary = i == 0
            || !hay[i - 1].is_ascii_alphanumeric()
            || (hay[i - 1].is_ascii_lowercase() && hc.is_ascii_uppercase());
        if boundary {
            score += 3;
        }
        last_hit = Some(i);
        n += 1;
    }

    if n == needle.len() {
        // Prefer shorter haystacks when everything else ties.
        Some(score - (hay.len() as i32 / 8))
    } else {
        None
    }
}
