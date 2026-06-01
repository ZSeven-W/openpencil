//! Semantic node-diff for the commit-detail card — a Rust port of the TS
//! `diffDocuments` (`packages/pen-core/src/merge/node-diff.ts`) +
//! `engineDiff` summary aggregation (`apps/desktop/git/git-engine.ts`).
//!
//! Both `.op` blobs are parsed as generic `serde_json::Value`, indexed by
//! node `id`, and compared field-by-field with `children` stripped — exactly
//! mirroring the TS `stripChildren` + `jsonEqual` path. Kept dependency-free
//! of the canonical typed schema so a malformed/legacy file degrades to a
//! best-effort diff instead of failing to parse.

use std::collections::HashSet;

use op_editor_core::{CommitDiffPatch, CommitDiffSummary, CommitDiffView, GitCommitSummary};
use op_git::GitRepo;
use serde_json::Value;

/// One indexed node: its structural context (page / parent / index) plus its
/// atomic fields (the node JSON with `children` removed).
struct Indexed {
    id: String,
    page_id: Option<String>,
    parent_id: Option<String>,
    index: usize,
    fields: Value,
}

/// Read the commit's blob + its parent's blob and compute the semantic diff.
/// Returns the lazy view state the card renders (TS `DiffState`).
pub fn load_commit_diff(
    repo: &GitRepo,
    relpath: &str,
    commit: &GitCommitSummary,
) -> CommitDiffView {
    if commit.is_initial {
        return CommitDiffView::Initial;
    }
    let rev = &commit.short_hash;
    // The file at this commit. Absent (`None`) means the commit removed it
    // (or predates its creation) → diff an empty doc so the removals show.
    let current = match repo.blob_at_commit(rev, relpath) {
        Ok(Some(s)) => s,
        Ok(None) => "{}".to_string(),
        Err(e) => return CommitDiffView::Error(e.to_string()),
    };
    // The first parent's version of the file. Absent (`None`) means the file
    // was added in this commit → diff against an empty document (all adds). A
    // real git error (e.g. an unreachable rev) surfaces rather than masking
    // as a phantom all-adds diff.
    let base = match repo.blob_at_commit(&format!("{rev}^"), relpath) {
        Ok(Some(s)) => s,
        Ok(None) => "{}".to_string(),
        Err(e) => return CommitDiffView::Error(e.to_string()),
    };

    let next_doc: Value = match serde_json::from_str(&current) {
        Ok(v) => v,
        Err(e) => return CommitDiffView::Error(e.to_string()),
    };
    let base_doc: Value = match serde_json::from_str(&base) {
        Ok(v) => v,
        Err(e) => return CommitDiffView::Error(e.to_string()),
    };

    let summary = compute_commit_diff(&base_doc, &next_doc);
    if summary.patches.is_empty() {
        CommitDiffView::NoChanges
    } else {
        CommitDiffView::Ready(summary)
    }
}

/// Diff `base` → `next` and aggregate the summary (TS `diffDocuments` +
/// `engineDiff`). Public for unit tests.
pub fn compute_commit_diff(base: &Value, next: &Value) -> CommitDiffSummary {
    let base_nodes = index_nodes(base);
    let next_nodes = index_nodes(next);

    // Lookup maps keyed by id; the ordered id list preserves the TS walk
    // order (base ids first, then next-only ids) for a stable patch list.
    let base_map: std::collections::HashMap<&str, &Indexed> =
        base_nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let next_map: std::collections::HashMap<&str, &Indexed> =
        next_nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut seen: HashSet<&str> = HashSet::new();
    let mut order: Vec<&str> = Vec::new();
    for n in base_nodes.iter().chain(next_nodes.iter()) {
        if seen.insert(n.id.as_str()) {
            order.push(n.id.as_str());
        }
    }

    let mut summary = CommitDiffSummary::default();
    let mut frames: HashSet<String> = HashSet::new();
    for id in order {
        match (base_map.get(id), next_map.get(id)) {
            (None, Some(n)) => {
                summary.nodes_added += 1;
                if let Some(p) = &n.parent_id {
                    frames.insert(p.clone());
                }
                summary.patches.push(patch("add", id));
            }
            (Some(_), None) => {
                summary.nodes_removed += 1;
                summary.patches.push(patch("remove", id));
            }
            (Some(b), Some(n)) => {
                // `move` and `modify` are independent — one id may produce both.
                let moved =
                    b.parent_id != n.parent_id || b.page_id != n.page_id || b.index != n.index;
                if moved {
                    summary.nodes_modified += 1;
                    if let Some(p) = &n.parent_id {
                        frames.insert(p.clone());
                    }
                    summary.patches.push(patch("move", id));
                }
                if !json_eq(&b.fields, &n.fields) {
                    summary.nodes_modified += 1;
                    summary.patches.push(patch("modify", id));
                }
            }
            (None, None) => {}
        }
    }
    summary.frames_changed = frames.len() as u32;
    summary
}

fn patch(op: &str, node_id: &str) -> CommitDiffPatch {
    CommitDiffPatch {
        op: op.to_string(),
        node_id: node_id.to_string(),
    }
}

/// Walk a document into a flat list of indexed nodes (TS `indexNodesById`).
/// Handles both the `pages` shape and the legacy single-page `children` shape.
fn index_nodes(doc: &Value) -> Vec<Indexed> {
    let mut out = Vec::new();
    for (page_id, children) in all_pages(doc) {
        walk(children, page_id, None, &mut out);
    }
    out
}

/// Normalize a document into `(pageId, children)` pairs. A legacy `children`
/// document yields one synthetic page with `id = None` (TS `getAllPages`).
fn all_pages(doc: &Value) -> Vec<(Option<String>, &Vec<Value>)> {
    if let Some(pages) = doc.get("pages").and_then(Value::as_array) {
        if !pages.is_empty() {
            return pages
                .iter()
                .filter_map(|p| {
                    let id = p.get("id").and_then(Value::as_str).map(str::to_string);
                    p.get("children").and_then(Value::as_array).map(|c| (id, c))
                })
                .collect();
        }
    }
    match doc.get("children").and_then(Value::as_array) {
        Some(children) => vec![(None, children)],
        None => Vec::new(),
    }
}

fn walk(
    nodes: &[Value],
    page_id: Option<String>,
    parent_id: Option<String>,
    out: &mut Vec<Indexed>,
) {
    for (index, node) in nodes.iter().enumerate() {
        let Some(id) = node.get("id").and_then(Value::as_str) else {
            continue;
        };
        out.push(Indexed {
            id: id.to_string(),
            page_id: page_id.clone(),
            parent_id: parent_id.clone(),
            index,
            fields: strip_children(node),
        });
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            if !children.is_empty() {
                walk(children, page_id.clone(), Some(id.to_string()), out);
            }
        }
    }
}

/// A shallow copy of `node` with the `children` field removed (TS
/// `stripChildren`).
fn strip_children(node: &Value) -> Value {
    let mut copy = node.clone();
    if let Some(map) = copy.as_object_mut() {
        map.remove("children");
    }
    copy
}

/// Structural equality matching TS `jsonEqual`: object key order is ignored
/// (as `serde_json` already does) AND numbers compare by numeric value, so a
/// field serialized once as `1` and once as `1.0` is NOT a spurious change.
/// `serde_json::Value`'s own `PartialEq` distinguishes integer `1` from float
/// `1.0`, which would otherwise manufacture phantom `modify` patches across
/// mixed Rust/TS-serialized histories.
fn json_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(xf), Some(yf)) => xf == yf,
            _ => x == y,
        },
        (Value::Array(xs), Value::Array(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| json_eq(x, y))
        }
        (Value::Object(xo), Value::Object(yo)) => {
            xo.len() == yo.len()
                && xo
                    .iter()
                    .all(|(k, xv)| yo.get(k).is_some_and(|yv| json_eq(xv, yv)))
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ready(view: CommitDiffView) -> CommitDiffSummary {
        match view {
            CommitDiffView::Ready(s) => s,
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn detects_added_removed_modified_and_moved() {
        let base = json!({
            "pages": [{ "id": "p1", "children": [
                { "id": "a", "name": "A", "children": [ { "id": "c", "name": "C" } ] },
                { "id": "b", "name": "B" }
            ]}]
        });
        let next = json!({
            "pages": [{ "id": "p1", "children": [
                { "id": "a", "name": "A2" },                         // modified (name) + c removed from it
                { "id": "c", "name": "C" },                          // moved to top level
                { "id": "d", "name": "D" }                           // added
            ]}]
        });
        let s = compute_commit_diff(&base, &next);
        assert_eq!(s.nodes_added, 1, "d added");
        assert_eq!(s.nodes_removed, 1, "b removed");
        // a: modify (name) ; c: move (reparented). Both count as modified.
        assert_eq!(s.nodes_modified, 2);
        let ops: Vec<&str> = s.patches.iter().map(|p| p.op.as_str()).collect();
        assert!(ops.contains(&"add"));
        assert!(ops.contains(&"remove"));
        assert!(ops.contains(&"modify"));
        assert!(ops.contains(&"move"));
    }

    #[test]
    fn identical_documents_yield_no_patches() {
        let doc = json!({ "pages": [{ "id": "p1", "children": [ { "id": "a", "name": "A" } ] }] });
        let s = compute_commit_diff(&doc, &doc);
        assert!(s.patches.is_empty());
        assert_eq!(s.frames_changed, 0);
    }

    #[test]
    fn single_field_modify_matches_image_77() {
        let base = json!({ "pages": [{ "id": "p1", "children": [
            { "id": "form-login-text", "text": "Sign in" }
        ]}]});
        let next = json!({ "pages": [{ "id": "p1", "children": [
            { "id": "form-login-text", "text": "Log in" }
        ]}]});
        let s = ready(CommitDiffView::Ready(compute_commit_diff(&base, &next)));
        assert_eq!(s.nodes_modified, 1);
        assert_eq!(s.patches.len(), 1);
        assert_eq!(s.patches[0].op, "modify");
        assert_eq!(s.patches[0].node_id, "form-login-text");
    }

    #[test]
    fn integer_vs_float_number_is_not_a_modify() {
        // A field serialized once as `1` and once as `1.0` is the same value
        // (TS `jsonEqual` via JSON.stringify) — must not manufacture a modify.
        let base = json!({ "pages": [{ "id": "p1", "children": [ { "id": "a", "x": 1 } ] }] });
        let next = json!({ "pages": [{ "id": "p1", "children": [ { "id": "a", "x": 1.0 } ] }] });
        let s = compute_commit_diff(&base, &next);
        assert!(s.patches.is_empty(), "1 vs 1.0 must not diff");
    }

    #[test]
    fn legacy_children_shape_is_supported() {
        let base = json!({ "children": [ { "id": "x", "name": "X" } ] });
        let next = json!({ "children": [ { "id": "x", "name": "X" }, { "id": "y" } ] });
        let s = compute_commit_diff(&base, &next);
        assert_eq!(s.nodes_added, 1);
    }
}
