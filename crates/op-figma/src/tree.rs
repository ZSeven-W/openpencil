//! Figma document tree builder — ports `figma-tree-builder.ts`.
//! Turns the flat `nodeChanges` list into a parent-child
//! [`TreeNode`] tree, ordered by `parentIndex.position`.

use crate::figma_types::FigGuid;
use crate::kiwi::FigValue;
use std::collections::{HashMap, HashSet};

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

/// Build the parent→children adjacency map (children in node-change
/// order) and locate the `DOCUMENT` root key.
fn build_adjacency(
    node_changes: &[FigValue],
    by_key: &HashMap<String, FigValue>,
) -> (HashMap<String, Vec<String>>, Option<String>) {
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut root_key: Option<String> = None;
    for nc in node_changes {
        if nc.get_str("phase") == Some("REMOVED") {
            continue;
        }
        let Some(key) = nc.get("guid").and_then(guid_to_string) else {
            continue;
        };
        if !by_key.contains_key(&key) {
            continue;
        }
        if nc.get_str("type") == Some("DOCUMENT") {
            root_key = Some(key);
            continue;
        }
        if let Some(parent_key) = nc
            .get("parentIndex")
            .and_then(|p| p.get("guid"))
            .and_then(guid_to_string)
        {
            if by_key.contains_key(&parent_key) {
                children_of.entry(parent_key).or_default().push(key);
            }
        }
    }
    (children_of, root_key)
}

/// Recursively materialize a `TreeNode`; children sorted descending by
/// `parentIndex.position` (z-stacking order — first child topmost).
fn materialize(
    key: &str,
    by_key: &HashMap<String, FigValue>,
    children_of: &HashMap<String, Vec<String>>,
    depth: u32,
) -> TreeNode {
    let figma = by_key.get(key).cloned().unwrap_or(FigValue::Null);
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
    let (by_key, _order) = index_by_key(node_changes);
    let (children_of, root_key) = build_adjacency(node_changes, &by_key);
    let root_key = root_key?;
    Some(materialize(&root_key, &by_key, &children_of, 0))
}

/// Build orphan-rooted trees for clipboard data with no `DOCUMENT`
/// wrapper — roots are nodes whose parent is absent.
pub fn build_tree_for_clipboard(node_changes: &[FigValue]) -> Vec<TreeNode> {
    let (by_key, order) = index_by_key(node_changes);
    let (children_of, _) = build_adjacency(node_changes, &by_key);
    let attached: HashSet<&String> = children_of.values().flatten().collect();
    order
        .iter()
        .filter(|k| !attached.contains(k))
        .filter(|k| {
            by_key
                .get(*k)
                .map(|n| n.get_str("type") != Some("DOCUMENT"))
                .unwrap_or(false)
        })
        .map(|k| materialize(k, &by_key, &children_of, 0))
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
pub fn collect_symbol_tree(node: &TreeNode, map: &mut HashMap<String, TreeNode>) {
    if node.figma.get_str("type") == Some("SYMBOL") {
        if let Some(key) = node.figma.get("guid").and_then(guid_to_string) {
            map.insert(key, node.clone());
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
