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

use crate::figma_types::FigVec2;
use crate::kiwi::FigValue;
use crate::tree::{guid_to_string, TreeNode};
use std::collections::{HashMap, HashSet};

mod apply;
mod assignment;
#[cfg(test)]
mod assignment_tests;
mod fingerprint;
#[cfg(test)]
mod fingerprint_tests;
mod foreign_session;
#[cfg(test)]
mod tests;

use apply::{
    apply_to_node, flatten_dfs, local_id, strip_first_guid, virtual_guid_base, walk_virtual,
};
pub use assignment::seed_assignments_from_instances;
use assignment::{guessed_mapping_is_implausible, rescale_only};

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

/// Apply a SYMBOL instance's overrides + derived data onto a clone of the
/// SYMBOL subtree with a throwaway assignment cache.
#[cfg(test)]
pub(crate) fn apply_instance_overrides(
    symbol_node: &TreeNode,
    overrides: Option<&[FigValue]>,
    derived: Option<&[FigValue]>,
    instance_size: Option<FigVec2>,
) -> Vec<TreeNode> {
    let mut cache = HashMap::new();
    apply_instance_overrides_cached(symbol_node, overrides, derived, instance_size, &mut cache)
}

/// Apply overrides with a cross-instance assignment cache. Instances
/// of the same SYMBOL share one virtual-GUID space, so an assignment
/// pinned by one instance's evidence (`"sym|pk"` → node guid) is
/// reused verbatim by every later instance — including for signal-less
/// entries (bare `visible` / `name`) that could never be fingerprinted
/// on their own.
pub fn apply_instance_overrides_cached(
    symbol_node: &TreeNode,
    overrides: Option<&[FigValue]>,
    derived: Option<&[FigValue]>,
    instance_size: Option<FigVec2>,
    assignment_cache: &mut HashMap<String, String>,
) -> Vec<TreeNode> {
    let overrides = overrides.unwrap_or(&[]);
    let derived = derived.unwrap_or(&[]);

    // Fast path — nothing to apply: just rescale to the instance size.
    if derived.is_empty() && overrides.is_empty() {
        return rescale_only(symbol_node, instance_size);
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

        // Anchor detection: when the BASE pk's derived size matches the
        // symbol's own size, the numbering starts at the ROOT (base =
        // root, base+1 = first child) — the walk fallback must then use
        // the root-anchored walk, or every mapping shifts by one.
        let base_pk = format!("{session_id}:{first_local_id}");
        let root_anchored = derived_map
            .get(&base_pk)
            .and_then(|d| d.get("size"))
            .and_then(FigVec2::from_value)
            .zip(symbol_node.figma.get("size").and_then(FigVec2::from_value))
            .map(|(ds, ss)| {
                let close = |a: f64, b: f64| (a - b).abs() <= (0.03 * b.abs()).max(4.0);
                close(ds.x, ss.x) && close(ds.y, ss.y)
            })
            .unwrap_or(false);
        let walk_map: &HashMap<String, String> = if root_anchored {
            &root_pk_to_node
        } else {
            &full_pk_to_node
        };

        for (pk, ng) in walk_map {
            pk_to_node_guid.insert(pk.clone(), ng.clone());
        }

        // Merge every single-segment virtual pk into a VirtualEntry.
        let mut virt_pks: Vec<&String> = Vec::new();
        let mut seen_pk: HashSet<&String> = HashSet::new();
        for pk in derived_order.iter().chain(override_order.iter()) {
            if !pk.contains('/') && seen_pk.insert(pk) {
                virt_pks.push(pk);
            }
        }
        let lid_of = |pk: &str| -> f64 {
            pk.split(':')
                .nth(1)
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0)
        };
        let entries: Vec<fingerprint::VirtualEntry> = virt_pks
            .iter()
            .map(|pk| fingerprint::VirtualEntry {
                pk: (*pk).clone(),
                rel_idx: lid_of(pk) - first_local_id as f64,
                derived: derived_map.get(*pk).copied(),
                overrides: override_map.get(*pk).copied(),
            })
            .collect();
        let (with_signal, signal_less): (Vec<_>, Vec<_>) =
            entries.into_iter().partition(fingerprint::has_signal);

        // Signal-bearing entries: fingerprint assignment (the virtual
        // allocation order is not recoverable — see fingerprint.rs).
        let ratios = match (
            instance_size,
            symbol_node.figma.get("size").and_then(FigVec2::from_value),
        ) {
            (Some(inst), Some(sym)) if sym.x > 0.0 && sym.y > 0.0 => {
                (inst.x / sym.x, inst.y / sym.y)
            }
            _ => (1.0, 1.0),
        };
        // Cross-instance cache: pins established by earlier instances
        // of this SYMBOL apply verbatim and reserve their nodes.
        let cache_key = |pk: &str| format!("{root_guid}|{pk}");
        let mut pinned_nodes: HashSet<String> = HashSet::new();
        let apply_pin = |pk: &String,
                         ng: &String,
                         node_derived: &mut HashMap<String, FigValue>,
                         node_override: &mut HashMap<String, FigValue>,
                         pk_to_node_guid: &mut HashMap<String, String>| {
            pk_to_node_guid.insert(pk.clone(), ng.clone());
            if let Some(d) = derived_map.get(pk) {
                node_derived.insert(ng.clone(), (*d).clone());
            }
            if let Some(ov) = override_map.get(pk) {
                node_override.insert(ng.clone(), (*ov).clone());
            }
        };
        let mut unpinned_signal: Vec<fingerprint::VirtualEntry> = Vec::new();
        for e in with_signal {
            if let Some(ng) = assignment_cache.get(&cache_key(&e.pk)).cloned() {
                pinned_nodes.insert(ng.clone());
                apply_pin(
                    &e.pk,
                    &ng,
                    &mut node_derived,
                    &mut node_override,
                    &mut pk_to_node_guid,
                );
            } else {
                unpinned_signal.push(e);
            }
        }

        let candidates: Vec<&TreeNode> = flat_symbol[1..]
            .iter()
            .copied()
            .filter(|n| {
                n.figma
                    .get("guid")
                    .and_then(guid_to_string)
                    .map(|g| !pinned_nodes.contains(&g))
                    .unwrap_or(true)
            })
            .collect();
        // `None` = pair-count guard tripped, no scoring ran: everything
        // goes through the walk-order fallback below. `Some` = entries
        // missing from the map were rejected. HARD-signal rejections
        // (size / transform / text contradicted every candidate) drop
        // — walk-ordering them back would restore the corruption the
        // evidence disproved. Fill-hint-only rejections merely mean
        // no candidate carries such a fill (the override may be
        // ADDING one) — those keep the walk-order fallback.
        let mut walk_fallback: Vec<fingerprint::VirtualEntry> = signal_less;
        match fingerprint::assign(&unpinned_signal, &candidates, ratios) {
            Some(assigned) => {
                for e in unpinned_signal {
                    if let Some(ng) = assigned.get(&e.pk) {
                        assignment_cache.insert(cache_key(&e.pk), ng.clone());
                        apply_pin(
                            &e.pk,
                            ng,
                            &mut node_derived,
                            &mut node_override,
                            &mut pk_to_node_guid,
                        );
                    } else if fingerprint::has_hard_signal(&e)
                        || fingerprint::hint_matches_any(&e, &flat_symbol[1..])
                    {
                        // Rejected on contradicting evidence, or the
                        // hint's target exists but another entry won
                        // it — either way the walk-order guess must
                        // not survive into application or
                        // nested-head resolution.
                        pk_to_node_guid.remove(&e.pk);
                    } else {
                        walk_fallback.push(e);
                    }
                }
            }
            None => walk_fallback.extend(unpinned_signal),
        }

        // Signal-less entries (bare name / visible tweaks) — plus every
        // entry when the fingerprint guard bailed: cache pin first,
        // then walk-order fallback with the root-walk filter for
        // root-level overrides.
        let mut foreign: Vec<foreign_session::ForeignPk> = Vec::new();
        for e in &walk_fallback {
            let pk = &e.pk;
            if let Some(ng) = assignment_cache.get(&cache_key(pk)).cloned() {
                apply_pin(
                    pk,
                    &ng,
                    &mut node_derived,
                    &mut node_override,
                    &mut pk_to_node_guid,
                );
                continue;
            }
            if !walk_map.contains_key(pk) {
                let is_foreign = pk
                    .split(':')
                    .next()
                    .and_then(|s| s.parse::<u32>().ok())
                    .map(|sid| sid != session_id)
                    .unwrap_or(false);
                if is_foreign {
                    foreign.push(foreign_session::ForeignPk {
                        pk: pk.clone(),
                        demands_text: fingerprint::demands_text(e),
                        demands_instance: false,
                    });
                }
                continue;
            }
            if let Some(d) = derived_map.get(pk) {
                if let Some(ng) = walk_map.get(pk) {
                    if *ng != root_guid {
                        node_derived.entry(ng.clone()).or_insert((*d).clone());
                    }
                }
            }
            if let Some(ov) = override_map.get(pk) {
                if root_pk_to_node.get(pk).map(String::as_str) == Some(root_guid.as_str()) {
                    continue;
                }
                if let Some(ng) = walk_map.get(pk) {
                    if *ng != root_guid {
                        node_override.entry(ng.clone()).or_insert((*ov).clone());
                    }
                }
            }
        }

        // Nested-path HEADS in a foreign session are group members
        // too: they demand an INSTANCE node — a strong validation
        // constraint the single-segment entries can't provide.
        for pk in override_order.iter().chain(derived_order.iter()) {
            if !pk.contains('/') {
                continue;
            }
            let head = pk.split('/').next().unwrap_or("");
            if walk_map.contains_key(head) || guid_in_subtree.contains(head) {
                continue;
            }
            let is_foreign = head
                .split(':')
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .map(|sid| sid != session_id)
                .unwrap_or(false);
            if !is_foreign {
                continue;
            }
            if let Some(fp) = foreign.iter_mut().find(|f| f.pk == head) {
                fp.demands_instance = true;
            } else {
                foreign.push(foreign_session::ForeignPk {
                    pk: head.to_string(),
                    demands_text: false,
                    demands_instance: true,
                });
            }
        }

        // Foreign-session groups: apply only under a UNIQUE anchor
        // whose walk satisfies every demand (text → TEXT node, nested
        // head → INSTANCE node); with no anchor, fall back to the
        // uniform-family match (see foreign_session.rs). Neither →
        // safely dropped.
        for group in foreign_session::group_by_session(&foreign) {
            let (map, family_mode) = match foreign_session::resolve_group(&group, symbol_node) {
                Some(m) => (m, false),
                None => {
                    match foreign_session::resolve_uniform_family(
                        &group,
                        symbol_node,
                        &override_map,
                    ) {
                        Some(m) => (m, true),
                        None => continue,
                    }
                }
            };
            for pk in group.pks.iter().map(|fp| &fp.pk) {
                let Some(ng) = map.get(pk) else { continue };
                if *ng == root_guid {
                    continue;
                }
                if family_mode {
                    // The family pairing is order-ARBITRARY — safe only
                    // for the identical cosmetic payload it validated.
                    // Per-pk derived data and nested-head resolution
                    // must not ride it.
                    if let Some(ov) = override_map.get(pk) {
                        node_override.entry(ng.clone()).or_insert((*ov).clone());
                    }
                    continue;
                }
                pk_to_node_guid.insert(pk.clone(), ng.clone());
                if let Some(d) = derived_map.get(pk) {
                    node_derived.entry(ng.clone()).or_insert((*d).clone());
                }
                if let Some(ov) = override_map.get(pk) {
                    node_override.entry(ng.clone()).or_insert((*ov).clone());
                }
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

    // ── Guessed-mapping confidence gate ──────────────────────────────
    // Strategies 1-3 GUESS the derived→node mapping from walk order,
    // but Figma's virtual-GUID allocation order is not recoverable
    // from the tree, so a wrong guess lands transforms / sizes on the
    // wrong nodes (overlapping text). Derived sizes that grossly
    // contradict the mapped nodes' own sizes betray a bad guess: when
    // a severe MAJORITY of the comparable entries contradict (≥2 and
    // more than half), the whole guessed application is dropped in
    // favour of a plain rescale — a small geometry drift beats a
    // corrupted layout.
    if !use_direct
        && guessed_mapping_is_implausible(&node_derived, &flat_symbol, instance_size, symbol_node)
    {
        // Rescale-only fallback — but multi-segment entries whose head
        // names a REAL subtree node are mapping-independent, so they
        // still forward into that node's nested slots.
        let mut safe_nested_override: HashMap<String, Vec<FigValue>> = HashMap::new();
        let mut safe_nested_derived: HashMap<String, Vec<FigValue>> = HashMap::new();
        let collect_safe = |order: &[String],
                            map: &HashMap<String, &FigValue>,
                            out: &mut HashMap<String, Vec<FigValue>>| {
            for pk in order {
                if !pk.contains('/') {
                    continue;
                }
                let head = pk.split('/').next().unwrap_or("");
                if !guid_in_subtree.contains(head) {
                    continue;
                }
                if let Some(entry) = map.get(pk) {
                    if let Some(rest) = strip_first_guid(entry) {
                        out.entry(head.to_string()).or_default().push(rest);
                    }
                }
            }
        };
        collect_safe(&override_order, &override_map, &mut safe_nested_override);
        collect_safe(&derived_order, &derived_map, &mut safe_nested_derived);
        let rescaled = rescale_only(symbol_node, instance_size);
        if safe_nested_override.is_empty() && safe_nested_derived.is_empty() {
            return rescaled;
        }
        let empty: HashMap<String, FigValue> = HashMap::new();
        return rescaled
            .iter()
            .map(|c| {
                apply_to_node(
                    c,
                    &empty,
                    &empty,
                    &safe_nested_override,
                    &safe_nested_derived,
                )
            })
            .collect();
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
