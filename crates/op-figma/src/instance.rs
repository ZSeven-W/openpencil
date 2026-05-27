//! Component-instance override application — ports the
//! `applyInstanceOverrides` / `mergeSymbolProps` parts of
//! `frame-converter.ts`.
//!
//! Four resolution strategies, picked in TS order:
//!   0. Direct-GUID match (`guidPath` names a real subtree node).
//!   1. Exact-count fallback when `len1Derived.len() == flat_symbol.len()`.
//!   2. Virtual-GUID DFS when a `sessionID:firstLocalID` base is present —
//!      two parallel walks, one started at children + one started at the
//!      root, so root-level overrides can be filtered out.
//!   3. Index-mapping fallback over all derived entries.

use crate::common::round2;
use crate::figma_types::FigVec2;
use crate::kiwi::FigValue;
use crate::tree::{guid_to_string, TreeNode};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests;

/// Layout keys an instance inherits from its master SYMBOL.
const LAYOUT_KEYS: &[&str] = &[
    "stackMode",
    "stackSpacing",
    "stackPadding",
    "stackHorizontalPadding",
    "stackVerticalPadding",
    "stackPaddingRight",
    "stackPaddingBottom",
    "stackPrimaryAlignItems",
    "stackCounterAlignItems",
    "stackPrimarySizing",
    "stackCounterSizing",
    "stackChildPrimaryGrow",
    "stackChildAlignSelf",
    "frameMaskDisabled",
];

/// Visual keys an instance inherits from its master SYMBOL.
const VISUAL_KEYS: &[&str] = &[
    "fillPaints",
    "strokePaints",
    "strokeWeight",
    "strokeAlign",
    "cornerRadius",
    "rectangleCornerRadiiIndependent",
    "rectangleTopLeftCornerRadius",
    "rectangleTopRightCornerRadius",
    "rectangleBottomLeftCornerRadius",
    "rectangleBottomRightCornerRadius",
];

/// Keys that must never be copied off an override entry onto a node.
const OVERRIDE_SKIP_KEYS: &[&str] = &[
    "guidPath",
    "guid",
    "parentIndex",
    "type",
    "phase",
    "symbolData",
    "derivedSymbolData",
    "componentKey",
    "variableConsumptionMap",
    "parameterConsumptionMap",
    "prototypeInteractions",
    "styleIdForFill",
    "styleIdForStrokeFill",
    "styleIdForText",
    "overrideLevel",
    "componentPropAssignments",
    "proportionsConstrained",
    "fontVersion",
];

/// Copy SYMBOL props onto an instance where the instance lacks them
/// (the instance's own values win).
pub fn merge_symbol_props(instance: &FigValue, symbol: &FigValue) -> FigValue {
    let mut merged = instance.clone();
    for key in LAYOUT_KEYS.iter().chain(VISUAL_KEYS.iter()) {
        if merged.get(key).is_none() {
            if let Some(v) = symbol.get(key) {
                merged.set(key, v.clone());
            }
        }
    }
    merged
}

/// `guidPath.guids` joined into a `/`-separated path key.
fn guid_path_key(entry: &FigValue) -> Option<String> {
    let guids = entry.get("guidPath")?.get_array("guids")?;
    if guids.is_empty() {
        return None;
    }
    let parts: Vec<String> = guids.iter().filter_map(guid_to_string).collect();
    if parts.len() == guids.len() {
        Some(parts.join("/"))
    } else {
        None
    }
}

/// Apply a SYMBOL instance's overrides + derived data onto a clone of
/// the SYMBOL subtree, returning the modified children.
pub fn apply_instance_overrides(
    symbol_node: &TreeNode,
    overrides: Option<&[FigValue]>,
    derived: Option<&[FigValue]>,
    instance_size: Option<FigVec2>,
) -> Vec<TreeNode> {
    let overrides = overrides.unwrap_or(&[]);
    let derived = derived.unwrap_or(&[]);

    // Fast path — nothing to apply: just rescale to the instance size.
    if derived.is_empty() && overrides.is_empty() {
        if let (Some(size), Some(Some(sym_size))) = (
            instance_size,
            symbol_node.figma.get("size").map(FigVec2::from_value),
        ) {
            if sym_size.x != 0.0 && sym_size.y != 0.0 {
                let sx = size.x / sym_size.x;
                let sy = size.y / sym_size.y;
                return crate::common::scale_tree_children(&symbol_node.children, sx, sy);
            }
        }
        return symbol_node.children.clone();
    }

    // ── Index inputs ─────────────────────────────────────────────────
    // override_map / derived_map are keyed by the full guidPathKey
    // ("`sid:lid`" for single-segment, "`sid:lid/sid2:lid2/...`" for
    // multi-segment).
    let mut override_map: HashMap<String, &FigValue> = HashMap::new();
    let mut override_order: Vec<String> = Vec::new();
    for entry in overrides {
        if let Some(key) = guid_path_key(entry) {
            if !override_map.contains_key(&key) {
                override_order.push(key.clone());
            }
            override_map.insert(key, entry);
        }
    }
    let mut derived_map: HashMap<String, &FigValue> = HashMap::new();
    let mut derived_order: Vec<String> = Vec::new();
    for entry in derived {
        if let Some(key) = guid_path_key(entry) {
            if !derived_map.contains_key(&key) {
                derived_order.push(key.clone());
            }
            derived_map.insert(key, entry);
        }
    }

    // Pre-order DFS of the symbol subtree, children sorted by ascending
    // localID (matches TS `flattenDFS(symbolNode)`).
    let mut flat_symbol: Vec<&TreeNode> = Vec::new();
    flatten_dfs(symbol_node, &mut flat_symbol);

    // Single-segment derived entries — these are the candidates for
    // the directMatches probe and Strategy-1 count match.
    let len1_derived: Vec<&FigValue> = derived
        .iter()
        .filter(|d| {
            d.get("guidPath")
                .and_then(|p| p.get_array("guids"))
                .map(|g| g.len() == 1)
                .unwrap_or(false)
        })
        .collect();

    // Set of guid-strings present in flat_symbol (used to count direct
    // hits + decide Strategy 0 vs 1/2/3).
    let mut guid_in_subtree: HashSet<String> = HashSet::new();
    for n in &flat_symbol {
        if let Some(k) = n.figma.get("guid").and_then(guid_to_string) {
            guid_in_subtree.insert(k);
        }
    }
    let direct_matches = len1_derived
        .iter()
        .filter_map(|d| {
            d.get("guidPath")
                .and_then(|p| p.get_array("guids"))
                .and_then(|g| g.first())
                .and_then(guid_to_string)
        })
        .filter(|k| guid_in_subtree.contains(k))
        .count();

    // Outputs of the resolution stage:
    //   node_override / node_derived — keyed by an actual subtree GUID.
    //   pk_to_node_guid — `guidPathKey first segment` → actual subtree GUID.
    let mut node_override: HashMap<String, FigValue> = HashMap::new();
    let mut node_derived: HashMap<String, FigValue> = HashMap::new();
    let mut pk_to_node_guid: HashMap<String, String> = HashMap::new();

    let use_direct = !len1_derived.is_empty() && direct_matches * 2 > len1_derived.len()
        || len1_derived.is_empty();

    if use_direct {
        // Strategy 0 — pkStr names a real subtree node.
        for d in &len1_derived {
            let Some(first) = d
                .get("guidPath")
                .and_then(|p| p.get_array("guids"))
                .and_then(|g| g.first())
                .and_then(guid_to_string)
            else {
                continue;
            };
            if guid_in_subtree.contains(&first) {
                if let Some(dv) = derived_map.get(&first) {
                    node_derived.insert(first.clone(), (*dv).clone());
                }
                if let Some(ov) = override_map.get(&first) {
                    node_override.insert(first.clone(), (*ov).clone());
                }
                pk_to_node_guid.insert(first.clone(), first);
            }
        }
        // Pick up single-segment override entries that reference real
        // node GUIDs even when no derived entry exists for them.
        for (pk, ov) in &override_map {
            if pk.contains('/') {
                continue;
            }
            if guid_in_subtree.contains(pk) {
                node_override.insert(pk.clone(), (*ov).clone());
            }
        }
    } else if len1_derived.len() == flat_symbol.len() {
        // Strategy 1 — exact count, index mapping.
        for (i, node) in flat_symbol.iter().enumerate() {
            let d = len1_derived[i];
            let Some(node_guid) = node.figma.get("guid").and_then(guid_to_string) else {
                continue;
            };
            let Some(pk) = guid_path_key(d) else { continue };
            // Map pkStr first-segment → actual node guid (used for
            // nested forwarding lookups).
            if let Some(first) = d
                .get("guidPath")
                .and_then(|p| p.get_array("guids"))
                .and_then(|g| g.first())
                .and_then(guid_to_string)
            {
                pk_to_node_guid.insert(first, node_guid.clone());
            }
            if let Some(dv) = derived_map.get(&pk) {
                node_derived.insert(node_guid.clone(), (*dv).clone());
            }
            if let Some(ov) = override_map.get(&pk) {
                node_override.insert(node_guid, (*ov).clone());
            }
        }
    } else if let Some((session_id, first_local_id)) = virtual_guid_base(&len1_derived) {
        // Strategy 2 — virtual-GUID DFS.
        // Two walks: `full` starts at children (sorted by localID);
        // `root` starts at symbol_node itself.
        let mut child_sorted: Vec<&TreeNode> = symbol_node.children.iter().collect();
        child_sorted.sort_by_key(|n| local_id(n));
        let mut full_pk_to_node: HashMap<String, String> = HashMap::new();
        let mut full_idx: u32 = 0;
        for c in &child_sorted {
            walk_virtual(
                c,
                session_id,
                first_local_id,
                &mut full_idx,
                &mut full_pk_to_node,
            );
        }
        let mut root_pk_to_node: HashMap<String, String> = HashMap::new();
        let mut root_idx: u32 = 0;
        walk_virtual(
            symbol_node,
            session_id,
            first_local_id,
            &mut root_idx,
            &mut root_pk_to_node,
        );

        let root_guid = symbol_node
            .figma
            .get("guid")
            .and_then(guid_to_string)
            .unwrap_or_default();

        for (pk, ng) in &full_pk_to_node {
            pk_to_node_guid.insert(pk.clone(), ng.clone());
        }
        for (pk, d) in &derived_map {
            if pk.contains('/') {
                continue;
            }
            if let Some(ng) = full_pk_to_node.get(pk) {
                node_derived.insert(ng.clone(), (*d).clone());
            }
        }
        for (pk, ov) in &override_map {
            if pk.contains('/') {
                continue;
            }
            if root_pk_to_node.get(pk).map(String::as_str) == Some(root_guid.as_str()) {
                continue;
            }
            if let Some(ng) = full_pk_to_node.get(pk) {
                node_override.insert(ng.clone(), (*ov).clone());
            }
        }
    } else {
        // Strategy 3 — fallback index mapping over all derived entries.
        let take = flat_symbol.len().min(derived.len());
        for i in 0..take {
            let node = flat_symbol[i];
            let d = &derived[i];
            let Some(node_guid) = node.figma.get("guid").and_then(guid_to_string) else {
                continue;
            };
            let Some(pk) = guid_path_key(d) else { continue };
            let single = d
                .get("guidPath")
                .and_then(|p| p.get_array("guids"))
                .map(|g| g.len() == 1)
                .unwrap_or(false);
            if let Some(dv) = derived_map.get(&pk) {
                node_derived.insert(node_guid.clone(), (*dv).clone());
            }
            if let Some(ov) = override_map.get(&pk) {
                node_override.insert(node_guid.clone(), (*ov).clone());
            }
            if single {
                if let Some(first) = d
                    .get("guidPath")
                    .and_then(|p| p.get_array("guids"))
                    .and_then(|g| g.first())
                    .and_then(guid_to_string)
                {
                    pk_to_node_guid.insert(first, node_guid);
                }
            }
        }
    }

    // ── Nested forwarding ────────────────────────────────────────────
    // Multi-segment guidPaths are forwarded into the resolved INSTANCE
    // node (head segment) as freshly-keyed entries.
    let mut nested_override: HashMap<String, Vec<FigValue>> = HashMap::new();
    let mut nested_derived: HashMap<String, Vec<FigValue>> = HashMap::new();
    for pk in &override_order {
        if !pk.contains('/') {
            continue;
        }
        let head = pk.split('/').next().unwrap_or("");
        let instance_guid = pk_to_node_guid
            .get(head)
            .cloned()
            .unwrap_or_else(|| head.to_string());
        if let Some(ov) = override_map.get(pk) {
            if let Some(rest) = strip_first_guid(ov) {
                nested_override.entry(instance_guid).or_default().push(rest);
            }
        }
    }
    for pk in &derived_order {
        if !pk.contains('/') {
            continue;
        }
        let head = pk.split('/').next().unwrap_or("");
        let instance_guid = pk_to_node_guid
            .get(head)
            .cloned()
            .unwrap_or_else(|| head.to_string());
        if let Some(d) = derived_map.get(pk) {
            if let Some(rest) = strip_first_guid(d) {
                nested_derived.entry(instance_guid).or_default().push(rest);
            }
        }
    }

    symbol_node
        .children
        .iter()
        .map(|c| {
            apply_to_node(
                c,
                &node_override,
                &node_derived,
                &nested_override,
                &nested_derived,
            )
        })
        .collect()
}

/// Local-id getter — falls back to 0 when `guid.localID` is absent
/// (matches TS `a.figma.guid?.localID ?? 0`).
fn local_id(node: &TreeNode) -> u32 {
    node.figma
        .get("guid")
        .and_then(|g| g.get_f64("localID"))
        .map(|n| n as u32)
        .unwrap_or(0)
}

/// Pre-order DFS over a TreeNode (children sorted ascending by
/// localID). The starting node is included as the first entry.
fn flatten_dfs<'a>(node: &'a TreeNode, out: &mut Vec<&'a TreeNode>) {
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
fn walk_virtual(
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
fn virtual_guid_base(len1_derived: &[&FigValue]) -> Option<(u32, u32)> {
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
fn strip_first_guid(entry: &FigValue) -> Option<FigValue> {
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
fn apply_to_node(
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

    // Forward nested entries into nested INSTANCE nodes.
    let is_instance =
        figma.get_str("type") == Some("INSTANCE") || figma.get("symbolData").is_some();
    if is_instance {
        if let Some(nested) = nested_ov {
            let mut existing: Vec<FigValue> = figma
                .get("symbolData")
                .and_then(|s| s.get_array("symbolOverrides"))
                .map(|a| a.to_vec())
                .unwrap_or_default();
            existing.extend(nested.clone());
            let mut symbol_data = figma
                .get("symbolData")
                .cloned()
                .unwrap_or(FigValue::Object(Vec::new()));
            symbol_data.set("symbolOverrides", FigValue::Array(existing));
            figma.set("symbolData", symbol_data);
        }
        if let Some(nested) = nested_d {
            figma.set("derivedSymbolData", FigValue::Array(nested.clone()));
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
