//! Deterministic cleanup for weak-model rebuild-and-abandon at page-root level.

use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use std::collections::{BTreeMap, BTreeSet};

const ABANDONED_DESCENDANT_RATIO: f64 = 0.30;
const MIN_FULLSCREEN_WIDTH: f64 = 320.0;
const MIN_FULLSCREEN_HEIGHT: f64 = 500.0;

#[derive(Debug, Default)]
pub(crate) struct DuplicateRootRemoval {
    removed_to_kept: BTreeMap<String, String>,
}

impl DuplicateRootRemoval {
    pub(crate) fn kept_for_removed(&self, removed_id: &str) -> Option<&str> {
        self.removed_to_kept.get(removed_id).map(String::as_str)
    }
}

#[derive(Debug, Clone)]
struct RootCandidate {
    id: String,
    name: Option<String>,
    descendants: usize,
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Remove sparse duplicate top-level artboard roots that a model rebuilt and
/// abandoned over the real root. The scope is deliberately only the active
/// page's top-level frame children.
pub(crate) fn remove_abandoned_duplicate_roots(sink: &mut dyn DocSink) -> DuplicateRootRemoval {
    let candidates: Vec<RootCandidate> = sink
        .state()
        .active_children()
        .iter()
        .filter_map(root_candidate)
        .collect();
    if candidates.len() < 2 {
        return DuplicateRootRemoval::default();
    }

    let mut removed_to_best: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for i in 0..candidates.len() {
        for j in (i + 1)..candidates.len() {
            let a = &candidates[i];
            let b = &candidates[j];
            if !can_compare_roots(a, b) || !roots_overlap(a, b) {
                continue;
            }
            let Some((sparse, rich)) = sparse_vs_rich(a, b) else {
                continue;
            };
            let entry = removed_to_best
                .entry(sparse.id.clone())
                .or_insert_with(|| (rich.id.clone(), rich.descendants));
            if rich.descendants > entry.1 {
                *entry = (rich.id.clone(), rich.descendants);
            }
        }
    }

    if removed_to_best.is_empty() {
        return DuplicateRootRemoval::default();
    }

    let mut removed_to_kept = BTreeMap::new();
    let mut deleted = BTreeSet::new();
    for (removed_id, (kept_id, _)) in removed_to_best {
        if removed_id == kept_id || !deleted.insert(removed_id.clone()) {
            continue;
        }
        sink.apply(EditorCommand::DeleteNode {
            node_id: NodeId::new(removed_id.clone()),
            page_id: None,
        });
        removed_to_kept.insert(removed_id, kept_id);
    }

    DuplicateRootRemoval { removed_to_kept }
}

fn root_candidate(node: &PenNode) -> Option<RootCandidate> {
    if !matches!(node, PenNode::Frame(_)) {
        return None;
    }
    let raw_name = node.base().name.as_deref();
    let name = raw_name
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string);
    Some(RootCandidate {
        id: node.id_str().to_string(),
        name,
        descendants: crate::cleanup::count_descendants(node),
        x: node.base().x,
        y: node.base().y,
        width: node.width_px(),
        height: node.height_px(),
    })
}

fn can_compare_roots(a: &RootCandidate, b: &RootCandidate) -> bool {
    match (&a.name, &b.name) {
        (Some(an), Some(bn)) => an == bn,
        (None, None) => is_fullscreen_root(a) && is_fullscreen_root(b),
        _ => false,
    }
}

fn sparse_vs_rich<'a>(
    a: &'a RootCandidate,
    b: &'a RootCandidate,
) -> Option<(&'a RootCandidate, &'a RootCandidate)> {
    if a.descendants == b.descendants {
        return None;
    }
    let (sparse, rich) = if a.descendants < b.descendants {
        (a, b)
    } else {
        (b, a)
    };
    if rich.descendants == 0 {
        return None;
    }
    ((sparse.descendants as f64) < (rich.descendants as f64 * ABANDONED_DESCENDANT_RATIO))
        .then_some((sparse, rich))
}

fn roots_overlap(a: &RootCandidate, b: &RootCandidate) -> bool {
    if !has_authored_position(a) && !has_authored_position(b) {
        return true;
    }
    if same_origin(a, b) {
        return true;
    }
    match (rect(a), rect(b)) {
        (Some(ar), Some(br)) => rects_intersect(ar, br),
        _ => false,
    }
}

fn has_authored_position(root: &RootCandidate) -> bool {
    root.x.is_some() || root.y.is_some()
}

fn same_origin(a: &RootCandidate, b: &RootCandidate) -> bool {
    (a.x.unwrap_or(0.0) - b.x.unwrap_or(0.0)).abs() <= 0.5
        && (a.y.unwrap_or(0.0) - b.y.unwrap_or(0.0)).abs() <= 0.5
}

fn rect(root: &RootCandidate) -> Option<Rect> {
    let w = root.width?;
    let h = root.height?;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some(Rect {
        x: root.x.unwrap_or(0.0),
        y: root.y.unwrap_or(0.0),
        w,
        h,
    })
}

fn rects_intersect(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

fn is_fullscreen_root(root: &RootCandidate) -> bool {
    root.width
        .is_some_and(|width| width >= MIN_FULLSCREEN_WIDTH)
        && root
            .height
            .is_some_and(|height| height >= MIN_FULLSCREEN_HEIGHT)
}
