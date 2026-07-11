//! Figma document tree builder — ports `figma-tree-builder.ts`.
//! Turns the flat `nodeChanges` list into a parent-child
//! [`TreeNode`] tree, ordered by `parentIndex.position`.

use crate::figma_types::FigGuid;
use crate::kiwi::FigValue;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Recursion ceiling for `materialize` — guards a malformed file with
/// a cyclic parent chain.
const MAX_TREE_DEPTH: u32 = 1024;

/// One node of the built document tree — a decoded node change plus
/// its ordered children.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub figma: FigValue,
    pub children: Vec<TreeNode>,
}

/// Canonical `sessionID:localID` key for a decoded `guid` object.
pub fn guid_to_string(guid: &FigValue) -> Option<String> {
    FigGuid::from_value(guid).map(|g| g.to_key())
}

/// True for a real, user-visible page — a `CANVAS` whose name is not
/// `Internal Only…` (matches `/^Internal\s+Only/i`).
pub fn is_user_page(node: &TreeNode) -> bool {
    if node.figma.get_str("type") != Some("CANVAS") {
        return false;
    }
    !is_internal_only(node.figma.get_str("name").unwrap_or(""))
}

fn is_internal_only(name: &str) -> bool {
    let lower = name.to_lowercase();
    let Some(rest) = lower.strip_prefix("internal") else {
        return false;
    };
    let trimmed = rest.trim_start();
    // Needs ≥1 whitespace char between "Internal" and "Only".
    trimmed.len() != rest.len() && trimmed.starts_with("only")
}

fn position_of(node: &FigValue) -> &str {
    node.get("parentIndex")
        .and_then(|p| p.get_str("position"))
        .unwrap_or("")
}

/// Index the node changes by guid key, in first-seen order, skipping
/// removed / guid-less entries.
fn index_by_key(node_changes: &[FigValue]) -> (HashMap<String, FigValue>, Vec<String>) {
    let mut by_key: HashMap<String, FigValue> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for nc in node_changes {
        if nc.get_str("phase") == Some("REMOVED") {
            continue;
        }
        let Some(key) = nc.get("guid").and_then(guid_to_string) else {
            continue;
        };
        if !by_key.contains_key(&key) {
            order.push(key.clone());
        }
        by_key.insert(key, nc.clone());
    }
    (by_key, order)
}

/// Build the parent→children adjacency map (children in first-seen-key
/// order) and locate the `DOCUMENT` root key.
///
/// Walks `order` — the deduplicated, first-seen-key list `index_by_key`
/// already produced — rather than the raw `node_changes` list, and reads
/// each key's edge data from `by_key` (the latest-wins record for that
/// guid). A `.fig` file may carry more than one change record for the
/// same guid (a legal, non-cyclic occurrence); re-scanning raw
/// `node_changes` here would push that guid into `children_of` once per
/// record — often under different parents — so `materialize`'s
/// single-parent-slot assumption sees the same key twice and the second
/// visit (after the first `remove()`s it from `by_key`) silently
/// degrades to `FigValue::Null`. Iterating the deduped `order` list
/// instead guarantees exactly one edge per guid, decided by that guid's
/// latest record — consistent with `index_by_key`'s latest-wins rule for
/// content.
fn build_adjacency(
    order: &[String],
    by_key: &HashMap<String, FigValue>,
) -> (HashMap<String, Vec<String>>, Option<String>) {
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut root_key: Option<String> = None;
    for key in order {
        let Some(nc) = by_key.get(key) else {
            continue;
        };
        if nc.get_str("type") == Some("DOCUMENT") {
            root_key = Some(key.clone());
            continue;
        }
        if let Some(parent_key) = nc
            .get("parentIndex")
            .and_then(|p| p.get("guid"))
            .and_then(guid_to_string)
        {
            if by_key.contains_key(&parent_key) {
                children_of.entry(parent_key).or_default().push(key.clone());
            }
        }
    }
    (children_of, root_key)
}

/// Recursively materialize a `TreeNode`; children sorted descending by
/// `parentIndex.position` (z-stacking order — first child topmost).
///
/// `by_key` is drained via `remove` rather than cloned: each guid is
/// normally visited exactly once (the adjacency map gives every node a
/// single parent slot), so ownership can move straight from the index
/// into the tree instead of paying for a second full-file clone. A
/// malformed file with a genuine parent cycle can revisit the same key
/// before `MAX_TREE_DEPTH` cuts the recursion off; a second visit then
/// sees an already-drained entry and falls back to `FigValue::Null`
/// (unchanged well-formed-file behavior; only the pathological-cycle
/// edge case differs from the old always-clone version).
fn materialize(
    key: &str,
    by_key: &mut HashMap<String, FigValue>,
    children_of: &HashMap<String, Vec<String>>,
    depth: u32,
) -> TreeNode {
    let figma = by_key.remove(key).unwrap_or(FigValue::Null);
    let mut children = Vec::new();
    if depth < MAX_TREE_DEPTH {
        if let Some(child_keys) = children_of.get(key) {
            let mut sorted: Vec<&String> = child_keys.iter().collect();
            sorted.sort_by(|a, b| {
                let pa = by_key.get(*a).map(position_of).unwrap_or("");
                let pb = by_key.get(*b).map(position_of).unwrap_or("");
                pb.cmp(pa) // descending
            });
            for ck in sorted {
                children.push(materialize(ck, by_key, children_of, depth + 1));
            }
        }
    }
    TreeNode { figma, children }
}

/// Build the canonical document tree rooted at the `DOCUMENT` node.
pub fn build_tree(node_changes: &[FigValue]) -> Option<TreeNode> {
    let (mut by_key, order) = index_by_key(node_changes);
    let (children_of, root_key) = build_adjacency(&order, &by_key);
    let root_key = root_key?;
    Some(materialize(&root_key, &mut by_key, &children_of, 0))
}

/// Build orphan-rooted trees for clipboard data with no `DOCUMENT`
/// wrapper — roots are nodes whose parent is absent.
pub fn build_tree_for_clipboard(node_changes: &[FigValue]) -> Vec<TreeNode> {
    let (mut by_key, order) = index_by_key(node_changes);
    let (children_of, _) = build_adjacency(&order, &by_key);
    let attached: HashSet<&String> = children_of.values().flatten().collect();
    // Resolve the root-key filter (reads `by_key`) fully before
    // `materialize` starts draining it, so the two phases don't need
    // simultaneous conflicting borrows of the same map.
    let root_keys: Vec<String> = order
        .into_iter()
        .filter(|k| !attached.contains(k))
        .filter(|k| {
            by_key
                .get(k)
                .map(|n| n.get_str("type") != Some("DOCUMENT"))
                .unwrap_or(false)
        })
        .collect();
    root_keys
        .iter()
        .map(|k| materialize(k, &mut by_key, &children_of, 0))
        .collect()
}

/// Pre-order DFS: register every `SYMBOL` node's guid → a freshly
/// minted `fig_N` id in `map`.
pub fn collect_components(node: &TreeNode, map: &mut HashMap<String, String>, counter: &mut u32) {
    if node.figma.get_str("type") == Some("SYMBOL") {
        if let Some(key) = node.figma.get("guid").and_then(guid_to_string) {
            let id = format!("fig_{}", *counter);
            *counter += 1;
            map.insert(key, id);
        }
    }
    for child in &node.children {
        collect_components(child, map, counter);
    }
}

/// Pre-order DFS: register every `SYMBOL` node's guid → its subtree.
///
/// The subtree is cloned once per distinct SYMBOL definition (the tree
/// walk that builds the page output still needs its own copy of the
/// master component), but wrapped in an `Rc` so every instance of that
/// SYMBOL shares the one clone — a cheap refcount bump per instance
/// instead of a second deep clone (previously the dominant cost on
/// instance-heavy files).
pub fn collect_symbol_tree(node: &TreeNode, map: &mut HashMap<String, Rc<TreeNode>>) {
    if node.figma.get_str("type") == Some("SYMBOL") {
        if let Some(key) = node.figma.get("guid").and_then(guid_to_string) {
            map.insert(key, Rc::new(node.clone()));
        }
    }
    for child in &node.children {
        collect_symbol_tree(child, map);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(pairs: Vec<(&str, FigValue)>) -> FigValue {
        FigValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn guid(s: u32, l: u32) -> FigValue {
        obj(vec![
            ("sessionID", FigValue::Uint(s)),
            ("localID", FigValue::Uint(l)),
        ])
    }

    fn node(local: u32, ty: &str, parent: Option<u32>, position: &str) -> FigValue {
        let mut pairs = vec![
            ("guid", guid(0, local)),
            ("type", FigValue::Str(ty.to_string())),
        ];
        if let Some(p) = parent {
            pairs.push((
                "parentIndex",
                obj(vec![
                    ("guid", guid(0, p)),
                    ("position", FigValue::Str(position.to_string())),
                ]),
            ));
        }
        obj(pairs)
    }

    #[test]
    fn builds_document_tree_with_sorted_children() {
        let changes = vec![
            node(0, "DOCUMENT", None, ""),
            node(1, "CANVAS", Some(0), "a"),
            // Two children of the canvas, positions "a" < "b".
            node(2, "RECTANGLE", Some(1), "a"),
            node(3, "TEXT", Some(1), "b"),
        ];
        let tree = build_tree(&changes).expect("tree builds");
        assert_eq!(tree.figma.get_str("type"), Some("DOCUMENT"));
        assert_eq!(tree.children.len(), 1);
        let canvas = &tree.children[0];
        // Descending position → "b" (TEXT) first.
        assert_eq!(canvas.children[0].figma.get_str("type"), Some("TEXT"));
        assert_eq!(canvas.children[1].figma.get_str("type"), Some("RECTANGLE"));
    }

    #[test]
    fn is_user_page_filters_internal() {
        let page = TreeNode {
            figma: obj(vec![
                ("type", FigValue::Str("CANVAS".into())),
                ("name", FigValue::Str("Internal Only Stuff".into())),
            ]),
            children: vec![],
        };
        assert!(!is_user_page(&page));
        let real = TreeNode {
            figma: obj(vec![
                ("type", FigValue::Str("CANVAS".into())),
                ("name", FigValue::Str("Page 1".into())),
            ]),
            children: vec![],
        };
        assert!(is_user_page(&real));
    }

    #[test]
    fn clipboard_roots_are_orphans() {
        let changes = vec![
            node(1, "FRAME", None, ""),
            node(2, "RECTANGLE", Some(1), "a"),
            node(3, "TEXT", None, ""),
        ];
        let roots = build_tree_for_clipboard(&changes);
        assert_eq!(roots.len(), 2); // FRAME + TEXT, RECTANGLE is nested
    }

    #[test]
    fn duplicate_guid_records_use_latest_parent_and_content_exactly_once() {
        // Two node-change records share guid 4: an earlier one parented
        // under FRAME 2 (position "x", name "v1") and a later one
        // re-parented under FRAME 3 (position "y", name "v2"). Legal,
        // non-cyclic `.fig` files can carry repeated guids like this
        // (e.g. a node moved during editing) — `index_by_key` already
        // picks the latest record's content; `build_adjacency` must
        // agree and wire exactly one parent edge from that same latest
        // record, instead of pushing the guid once per raw occurrence
        // in `node_changes`.
        let mut first = node(4, "RECTANGLE", Some(2), "x");
        if let FigValue::Object(pairs) = &mut first {
            pairs.push(("name".into(), FigValue::Str("v1".into())));
        }
        let mut second = node(4, "RECTANGLE", Some(3), "y");
        if let FigValue::Object(pairs) = &mut second {
            pairs.push(("name".into(), FigValue::Str("v2".into())));
        }
        let changes = vec![
            node(0, "DOCUMENT", None, ""),
            node(1, "CANVAS", Some(0), "a"),
            node(2, "FRAME", Some(1), "a"),
            node(3, "FRAME", Some(1), "b"),
            first,
            second,
        ];
        let tree = build_tree(&changes).expect("tree builds");
        let canvas = &tree.children[0];
        assert_eq!(canvas.children.len(), 2);
        // Descending position sort ("b" > "a") puts guid-3's FRAME first.
        let frame_b = &canvas.children[0];
        let frame_a = &canvas.children[1];

        // Node 4 must appear under its LATEST parent (frame 3) exactly
        // once, carrying the latest record's content ("v2") — never
        // under the earlier parent (frame 2), and never duplicated.
        assert!(
            frame_a.children.is_empty(),
            "node 4 must not remain under its stale parent"
        );
        assert_eq!(frame_b.children.len(), 1, "node 4 must appear exactly once");
        assert_eq!(frame_b.children[0].figma.get_str("name"), Some("v2"));

        // No node in the materialized tree silently degraded to Null —
        // the historical failure mode when a repeated guid revisited an
        // already-`remove()`d `by_key` entry.
        fn assert_no_null(n: &TreeNode) {
            assert!(
                !matches!(n.figma, FigValue::Null),
                "unexpected Null node in tree"
            );
            for c in &n.children {
                assert_no_null(c);
            }
        }
        assert_no_null(&tree);
    }

    #[test]
    fn removed_nodes_are_skipped() {
        let mut removed = node(2, "RECTANGLE", Some(1), "a");
        if let FigValue::Object(pairs) = &mut removed {
            pairs.push(("phase".into(), FigValue::Str("REMOVED".into())));
        }
        let changes = vec![
            node(0, "DOCUMENT", None, ""),
            node(1, "CANVAS", Some(0), "a"),
            removed,
        ];
        let tree = build_tree(&changes).unwrap();
        assert!(tree.children[0].children.is_empty());
    }
}
