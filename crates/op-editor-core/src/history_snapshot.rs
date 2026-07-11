//! Structurally-shared snapshot representation for undo / redo.
//!
//! Every undoable gesture used to deep-clone the entire `PenDocument`
//! (node tree + all component prototypes) into an [`crate::history::EditorSnapshot`],
//! and `HISTORY_CAP = 100` kept up to 100 such clones alive at once —
//! peak memory ~100× the document.
//!
//! This module replaces the snapshot's owned `PenDocument` /
//! `ComponentLibrary` with a **share-at-top-level-node granularity**
//! representation:
//!
//!   - [`SharedDoc`] keeps a children-stripped `PenDocument` *skeleton*
//!     (all scalar / collection fields — version, name, themes,
//!     variables, page metadata, app/state/lifecycle/… — cloned as
//!     today) plus, for the single-page root AND every page, the
//!     top-level nodes as `Vec<Arc<PenNode>>`.
//!   - [`SharedComponents`] keeps the component prototypes as
//!     `Vec<Arc<Component>>` (id-keyed sharing).
//!
//! On [`SharedDoc::capture`] each top-level entry is compared against
//! the *anchor* snapshot (the adjacent history state); an unchanged
//! subtree reuses the anchor's `Arc` (a pointer bump, no deep clone),
//! so 100 snapshots retain one copy of every subtree that never
//! changed plus the deltas. Matching is an index fast path (same slot,
//! same id, deep-equal) with an id-keyed fallback map for equal-content
//! duplicates — equality always gates reuse, so a reused `Arc` is
//! byte-identical to what a full clone would have produced.
//!
//! Snapshot-stored `Arc`s are treated **copy-on-write**: the one
//! in-place mutator of stored snapshots
//! ([`crate::instance_override`]'s `repair_scope_snapshots`) routes
//! through [`SharedDoc::repair_swap`], which `Arc::make_mut`s only the
//! affected top-level entry — cloning it away from any sibling snapshot
//! that shares it, so no other history state is contaminated.

use crate::components::{Component, ComponentLibrary};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::page::PenPage;
use jian_ops_schema::variable::VariableDefinition;
use jian_ops_schema::PenDocument;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// A structurally-shared mirror of a [`PenDocument`] for undo history.
///
/// The `skeleton` is a full `PenDocument` with every node-children
/// vector emptied (root `children` and each page's `children`); it
/// carries all the cheap scalar / collection fields verbatim. The
/// actual top-level nodes live in `root_children` / `page_children` as
/// shared `Arc`s, parallel to `skeleton.pages`.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedDoc {
    /// The document minus every node-children vector. Cheap to clone
    /// and, being a real `PenDocument`, robust against schema growth.
    skeleton: PenDocument,
    /// Shared top-level nodes of the single-page root `children`.
    root_children: Vec<Arc<PenNode>>,
    /// Shared top-level nodes per page — `page_children[i]` parallels
    /// `skeleton.pages[i]`.
    page_children: Vec<Vec<Arc<PenNode>>>,
}

impl SharedDoc {
    /// Capture `doc` into a shared snapshot, reusing `anchor`'s `Arc`s
    /// for any top-level subtree that is unchanged. `anchor` is the
    /// adjacent history state (see [`crate::state::EditorState::snapshot_for_history_with_anchor`]);
    /// `None` produces all-fresh `Arc`s.
    pub fn capture(doc: &PenDocument, anchor: Option<&SharedDoc>) -> SharedDoc {
        let root_children =
            share_children(&doc.children, anchor.map(|a| a.root_children.as_slice()));
        let mut page_children = Vec::new();
        if let Some(pages) = doc.pages.as_ref() {
            for (i, page) in pages.iter().enumerate() {
                let prev = anchor
                    .and_then(|a| a.page_children.get(i))
                    .map(|v| v.as_slice());
                page_children.push(share_children(&page.children, prev));
            }
        }
        SharedDoc {
            skeleton: strip_children(doc),
            root_children,
            page_children,
        }
    }

    /// Rebuild an owned [`PenDocument`] — deep-clones every shared node
    /// back into place. The materialized document is byte-identical to
    /// the one that was captured.
    pub fn materialize(&self) -> PenDocument {
        let mut doc = self.skeleton.clone();
        doc.children = self
            .root_children
            .iter()
            .map(|a| a.as_ref().clone())
            .collect();
        if let Some(pages) = doc.pages.as_mut() {
            for (page, shared) in pages.iter_mut().zip(self.page_children.iter()) {
                page.children = shared.iter().map(|a| a.as_ref().clone()).collect();
            }
        }
        doc
    }

    /// The document's variable table, as carried by the skeleton.
    pub fn variables(&self) -> Option<&BTreeMap<String, VariableDefinition>> {
        self.skeleton.variables.as_ref()
    }

    /// Find a node by id within the active page's shared children.
    /// Mirrors the (deliberately un-clamped) snapshot-reader semantics
    /// of the old `snapshot_active_children`: an out-of-range page
    /// index or an empty `pages` list resolves to nothing rather than
    /// falling back to the root `children`.
    pub fn snapshot_find_node(&self, active_page_index: usize, id: &NodeId) -> Option<&PenNode> {
        let children: &[Arc<PenNode>] = match self.skeleton.pages.as_ref() {
            Some(pages) => {
                if pages.get(active_page_index).is_some() {
                    self.page_children
                        .get(active_page_index)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[])
                } else {
                    &[]
                }
            }
            None => &self.root_children,
        };
        find_in_shared(children, id)
    }

    /// Copy-on-write repair used by
    /// `instance_override::repair_scope_snapshots`: for every top-level
    /// entry whose subtree holds a *contaminated* (non-`Ref`) node at
    /// `id`, `Arc::make_mut` the entry — isolating it from any sibling
    /// snapshot that shares the `Arc` — then swap the node for
    /// `replacement`. Entries without contamination keep their shared
    /// `Arc` untouched (their `ptr_eq` sharing is preserved).
    pub(crate) fn repair_swap(&mut self, id: &NodeId, replacement: &PenNode) {
        for entry in &mut self.root_children {
            repair_entry(entry, id, replacement);
        }
        for page in &mut self.page_children {
            for entry in page {
                repair_entry(entry, id, replacement);
            }
        }
    }

    /// Shared top-level nodes of the single-page root. Test / assertion
    /// hook for `ptr_eq` sharing checks.
    pub fn root_children(&self) -> &[Arc<PenNode>] {
        &self.root_children
    }

    /// Shared top-level nodes of page `i`, if any.
    pub fn page_children(&self, i: usize) -> Option<&[Arc<PenNode>]> {
        self.page_children.get(i).map(|v| v.as_slice())
    }
}

/// Component prototypes stored in a snapshot, shared by `Arc`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SharedComponents {
    components: Vec<Arc<Component>>,
}

impl SharedComponents {
    /// Capture the library, reusing `anchor`'s `Arc`s for unchanged
    /// prototypes (id + deep-equality gated).
    pub fn capture(lib: &ComponentLibrary, anchor: Option<&SharedComponents>) -> Self {
        SharedComponents {
            components: share_components(
                &lib.components,
                anchor.map(|a| a.components.as_slice()),
            ),
        }
    }

    /// Rebuild an owned [`ComponentLibrary`].
    pub fn materialize(&self) -> ComponentLibrary {
        ComponentLibrary {
            components: self
                .components
                .iter()
                .map(|a| a.as_ref().clone())
                .collect(),
        }
    }

    /// Shared prototypes. Test / assertion hook for `ptr_eq` checks.
    pub fn components(&self) -> &[Arc<Component>] {
        &self.components
    }
}

// --- Free helpers ----------------------------------------------------

/// A `PenDocument` with every node-children vector emptied — the
/// snapshot skeleton. Written as an explicit struct literal (no
/// `..Default`) so a new `PenDocument` field fails to compile here
/// until it is classified, keeping the skeleton schema-complete.
fn strip_children(doc: &PenDocument) -> PenDocument {
    PenDocument {
        version: doc.version.clone(),
        name: doc.name.clone(),
        themes: doc.themes.clone(),
        variables: doc.variables.clone(),
        pages: doc
            .pages
            .as_ref()
            .map(|pages| pages.iter().map(strip_page_children).collect()),
        children: Vec::new(),
        format_version: doc.format_version.clone(),
        id: doc.id.clone(),
        app: doc.app.clone(),
        routes: doc.routes.clone(),
        state: doc.state.clone(),
        lifecycle: doc.lifecycle.clone(),
        logic_modules: doc.logic_modules.clone(),
        design_md: doc.design_md.clone(),
        conversion: doc.conversion.clone(),
    }
}

/// A `PenPage` with its `children` emptied. Explicit literal, same
/// schema-completeness rationale as [`strip_children`].
fn strip_page_children(page: &PenPage) -> PenPage {
    PenPage {
        id: page.id.clone(),
        name: page.name.clone(),
        state: page.state.clone(),
        lifecycle: page.lifecycle.clone(),
        children: Vec::new(),
    }
}

/// Build the shared top-level vector for `current`, reusing `prev`'s
/// `Arc`s for equal entries. Index fast path (same slot + same id +
/// deep-equal) with a lazily-allocated id-keyed fallback map. The
/// equality check gates every reuse, so a shared `Arc` is guaranteed
/// content-identical to a fresh clone.
fn share_children(current: &[PenNode], prev: Option<&[Arc<PenNode>]>) -> Vec<Arc<PenNode>> {
    let Some(prev) = prev else {
        return current.iter().map(|n| Arc::new(n.clone())).collect();
    };
    let mut id_map: Option<HashMap<&str, &Arc<PenNode>>> = None;
    current
        .iter()
        .enumerate()
        .map(|(i, node)| {
            // Index fast path: same slot, same id, deep-equal.
            if let Some(prev_arc) = prev.get(i) {
                if prev_arc.id_str() == node.id_str() && prev_arc.as_ref() == node {
                    return Arc::clone(prev_arc);
                }
            }
            // Id-keyed fallback (allocates a small map — the
            // allocation-free claim applies to the index walk only).
            let map = id_map.get_or_insert_with(|| prev.iter().map(|a| (a.id_str(), a)).collect());
            if let Some(&prev_arc) = map.get(node.id_str()) {
                if prev_arc.as_ref() == node {
                    return Arc::clone(prev_arc);
                }
            }
            Arc::new(node.clone())
        })
        .collect()
}

/// [`share_children`] for component prototypes, keyed by `Component::id`.
fn share_components(
    current: &[Component],
    prev: Option<&[Arc<Component>]>,
) -> Vec<Arc<Component>> {
    let Some(prev) = prev else {
        return current.iter().map(|c| Arc::new(c.clone())).collect();
    };
    let mut id_map: Option<HashMap<&NodeId, &Arc<Component>>> = None;
    current
        .iter()
        .enumerate()
        .map(|(i, comp)| {
            if let Some(prev_arc) = prev.get(i) {
                if prev_arc.id == comp.id && prev_arc.as_ref() == comp {
                    return Arc::clone(prev_arc);
                }
            }
            let map = id_map.get_or_insert_with(|| prev.iter().map(|a| (&a.id, a)).collect());
            if let Some(&prev_arc) = map.get(&comp.id) {
                if prev_arc.as_ref() == comp {
                    return Arc::clone(prev_arc);
                }
            }
            Arc::new(comp.clone())
        })
        .collect()
}

/// Find a node by id among top-level shared `Arc`s (recursing into the
/// owned subtree of each entry).
fn find_in_shared<'a>(children: &'a [Arc<PenNode>], id: &NodeId) -> Option<&'a PenNode> {
    for arc in children {
        let node = arc.as_ref();
        if node.id_str() == id.as_str() {
            return Some(node);
        }
        if let Some(sub) = node.children() {
            if let Some(found) = crate::walkers::find_node(sub, id) {
                return Some(found);
            }
        }
    }
    None
}

/// COW-repair one top-level entry. `Arc::make_mut` runs only when the
/// entry actually holds a non-`Ref` node at `id`.
fn repair_entry(entry: &mut Arc<PenNode>, id: &NodeId, replacement: &PenNode) {
    let needs = {
        let node = entry.as_ref();
        let found = if node.id_str() == id.as_str() {
            Some(node)
        } else {
            node.children().and_then(|c| crate::walkers::find_node(c, id))
        };
        matches!(found, Some(n) if !matches!(n, PenNode::Ref(_)))
    };
    if !needs {
        return;
    }
    let node = Arc::make_mut(entry);
    let target = if node.id_str() == id.as_str() {
        Some(node)
    } else {
        node.children_mut()
            .and_then(|c| crate::walkers::find_node_mut(c, id))
    };
    if let Some(t) = target {
        if !matches!(t, PenNode::Ref(_)) {
            *t = replacement.clone();
        }
    }
}
