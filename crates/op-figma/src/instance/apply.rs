//! Subtree cloning + per-node application of resolved override /
//! derived data, plus the virtual-GUID walk helpers shared by the
//! Strategy-2 resolution and the foreign-session anchoring.

use super::{guid_path_key, OVERRIDE_SKIP_KEYS};
use crate::common::round2;
use crate::figma_types::FigVec2;
use crate::kiwi::FigValue;
use crate::tree::{guid_to_string, TreeNode};
use std::collections::HashMap;

/// Local-id getter — falls back to 0 when `guid.localID` is absent
/// (matches TS `a.figma.guid?.localID ?? 0`).
pub(super) fn local_id(node: &TreeNode) -> u32 {
    node.figma
        .get("guid")
        .and_then(|g| g.get_f64("localID"))
        .map(|n| n as u32)
        .unwrap_or(0)
}

/// Pre-order DFS over a TreeNode (children sorted ascending by
/// localID). The starting node is included as the first entry.
pub(super) fn flatten_dfs<'a>(node: &'a TreeNode, out: &mut Vec<&'a TreeNode>) {
    out.push(node);
    let mut sorted: Vec<&TreeNode> = node.children.iter().collect();
    sorted.sort_by_key(|n| local_id(n));
    for c in sorted {
        flatten_dfs(c, out);
    }
}

/// Walk a subtree in pre-order DFS, recording the virtual GUID
/// `sessionID:firstLocalID + idx` → actual GUID for each node. Mirrors
/// the TS `walkFull` / `walkRoot` helpers.
pub(super) fn walk_virtual(
    node: &TreeNode,
    session_id: u32,
    first_local_id: u32,
    idx: &mut u32,
    out: &mut HashMap<String, String>,
) {
    if let Some(g) = node.figma.get("guid").and_then(guid_to_string) {
        out.insert(format!("{}:{}", session_id, first_local_id + *idx), g);
    }
    *idx += 1;
    let mut sorted: Vec<&TreeNode> = node.children.iter().collect();
    sorted.sort_by_key(|n| local_id(n));
    for c in sorted {
        walk_virtual(c, session_id, first_local_id, idx, out);
    }
}

/// Read `(sessionID, firstLocalID)` from the first single-segment
/// derived entry — Strategy-2's virtual-GUID base. None when either
/// field is missing.
pub(super) fn virtual_guid_base(len1_derived: &[&FigValue]) -> Option<(u32, u32)> {
    let first = len1_derived.first()?;
    let first_guid = first
        .get("guidPath")
        .and_then(|p| p.get_array("guids"))
        .and_then(|g| g.first())?;
    let sid = first_guid.get_f64("sessionID")? as u32;
    let lid = first_guid.get_f64("localID")? as u32;
    Some((sid, lid))
}

/// Build a copy of `entry` with the first `guidPath` segment dropped.
pub(super) fn strip_first_guid(entry: &FigValue) -> Option<FigValue> {
    let guids = entry.get("guidPath")?.get_array("guids")?;
    if guids.len() < 2 {
        return None;
    }
    let mut copy = entry.clone();
    let rest: Vec<FigValue> = guids[1..].to_vec();
    let mut path = FigValue::Object(Vec::new());
    path.set("guids", FigValue::Array(rest));
    copy.set("guidPath", path);
    Some(copy)
}

/// Recursively clone the subtree, applying derived data + overrides to
/// each node keyed by its guid.
/// Merge forwarded override/derived entries into a node's authored
/// list: same-pk entries field-merge (authored fields win), new pks
/// append. Keeps the list free of duplicate pks — a duplicated pk
/// double-counts in the Strategy-1 length probe.
fn merge_entry_lists(existing: Vec<FigValue>, forwarded: &[FigValue]) -> Vec<FigValue> {
    let mut out = existing;
    for f in forwarded {
        let fk = guid_path_key(f);
        let slot = fk.as_ref().and_then(|k| {
            out.iter_mut()
                .find(|e| guid_path_key(e).as_ref() == Some(k))
        });
        match slot {
            Some(e) => {
                if let (FigValue::Object(epairs), FigValue::Object(fpairs)) = (&mut *e, f) {
                    for (k, v) in fpairs {
                        if k != "guidPath" && !epairs.iter().any(|(ek, _)| ek == k) {
                            epairs.push((k.clone(), v.clone()));
                        }
                    }
                }
            }
            None => out.push(f.clone()),
        }
    }
    out
}

pub(super) fn apply_to_node(
    node: &TreeNode,
    node_override: &HashMap<String, FigValue>,
    node_derived: &HashMap<String, FigValue>,
    nested_override: &HashMap<String, Vec<FigValue>>,
    nested_derived: &HashMap<String, Vec<FigValue>>,
) -> TreeNode {
    let key = node
        .figma
        .get("guid")
        .and_then(guid_to_string)
        .unwrap_or_default();
    let d = node_derived.get(&key);
    let ov = node_override.get(&key);
    let nested_ov = nested_override.get(&key);
    let nested_d = nested_derived.get(&key);

    if d.is_none() && ov.is_none() && nested_ov.is_none() && nested_d.is_none() {
        return TreeNode {
            figma: node.figma.clone(),
            children: node
                .children
                .iter()
                .map(|c| {
                    apply_to_node(
                        c,
                        node_override,
                        node_derived,
                        nested_override,
                        nested_derived,
                    )
                })
                .collect(),
        };
    }

    let mut figma = node.figma.clone();

    // Derived data — scale stroke weight before overwriting size.
    if let Some(d) = d {
        if let (Some(dsize), Some(nsize)) = (
            d.get("size").and_then(FigVec2::from_value),
            node.figma.get("size").and_then(FigVec2::from_value),
        ) {
            if let Some(sw) = figma.get_f64("strokeWeight") {
                if nsize.x != 0.0 && nsize.y != 0.0 {
                    let scale = (dsize.x / nsize.x).min(dsize.y / nsize.y);
                    if scale < 0.99 {
                        figma.set("strokeWeight", FigValue::Float(round2(sw * scale) as f32));
                    }
                }
            }
        }
        if let Some(size) = d.get("size") {
            figma.set("size", size.clone());
        }
        if let Some(t) = d.get("transform") {
            figma.set("transform", t.clone());
        }
        if let Some(fs) = d.get("fontSize") {
            figma.set("fontSize", fs.clone());
        }
        if let Some(dtd) = d.get("derivedTextData") {
            if dtd.get("characters").is_some() {
                figma.set("textData", dtd.clone());
            }
        }
    }

    // Override props — copy every non-blacklisted key. Explicit
    // `Null` is preserved (TS `if (value !== undefined)`: only
    // `undefined` is skipped, `null` is copied as an intentional
    // reset).
    if let Some(FigValue::Object(pairs)) = ov {
        for (k, v) in pairs {
            if !OVERRIDE_SKIP_KEYS.contains(&k.as_str()) {
                figma.set(k, v.clone());
            }
        }
    }

    // Forward nested entries into nested INSTANCE nodes. Forwarded
    // entries MERGE with the node's own authored lists — replacing
    // them would strip a nested icon's scale / fill targets, and
    // appending duplicates would inflate the entry count into a false
    // Strategy-1 (index-mapping) match downstream. Per pk, authored
    // fields win; forwarded-only fields fill the gaps.
    let is_instance =
        figma.get_str("type") == Some("INSTANCE") || figma.get("symbolData").is_some();
    if is_instance {
        if let Some(nested) = nested_ov {
            let existing: Vec<FigValue> = figma
                .get("symbolData")
                .and_then(|s| s.get_array("symbolOverrides"))
                .map(|a| a.to_vec())
                .unwrap_or_default();
            let merged = merge_entry_lists(existing, nested);
            let mut symbol_data = figma
                .get("symbolData")
                .cloned()
                .unwrap_or(FigValue::Object(Vec::new()));
            symbol_data.set("symbolOverrides", FigValue::Array(merged));
            figma.set("symbolData", symbol_data);
        }
        if let Some(nested) = nested_d {
            let existing: Vec<FigValue> = figma
                .get_array("derivedSymbolData")
                .map(|a| a.to_vec())
                .unwrap_or_default();
            let merged = merge_entry_lists(existing, nested);
            figma.set("derivedSymbolData", FigValue::Array(merged));
        }
    }

    TreeNode {
        figma,
        children: node
            .children
            .iter()
            .map(|c| {
                apply_to_node(
                    c,
                    node_override,
                    node_derived,
                    nested_override,
                    nested_derived,
                )
            })
            .collect(),
    }
}
