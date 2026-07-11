use super::{fingerprint, flatten_dfs};
use crate::common::scale_tree_children;
use crate::figma_types::FigVec2;
use crate::kiwi::FigValue;
use crate::tree::{guid_to_string, TreeNode};
use std::collections::HashMap;
use std::rc::Rc;

/// Pre-conversion pooled seeding: walk the whole tree, group every
/// INSTANCE's single-segment virtual entries by SYMBOL, pool their
/// text evidence per pk, and pin the decisive assignments into
/// `cache` (`"sym|pk"` -> node guid). Runs BEFORE any conversion so
/// pin quality doesn't depend on which instance converts first.
pub fn seed_assignments_from_instances(
    root: &TreeNode,
    symbol_tree: &HashMap<String, Rc<TreeNode>>,
    cache: &mut HashMap<String, String>,
) {
    // symbol guid -> pk -> pooled evidence.
    type Pool = HashMap<String, HashMap<String, (f64, Vec<String>, bool)>>;
    let mut pool: Pool = HashMap::new();

    fn collect(node: &TreeNode, pool: &mut Pool) {
        let figma = &node.figma;
        // A swapped instance's derived belongs to the swapped-in
        // component, not its base `symbolID`, so pooling it under the
        // base guid would poison genuine base-component instances.
        if figma.get_str("type") == Some("INSTANCE") && figma.get("overriddenSymbolID").is_none() {
            if let Some(sym_guid) = figma
                .get("symbolData")
                .and_then(|s| s.get("symbolID"))
                .and_then(|g| {
                    Some(format!(
                        "{}:{}",
                        g.get_f64("sessionID")? as u64,
                        g.get_f64("localID")? as u64
                    ))
                })
            {
                let per_sym = pool.entry(sym_guid).or_default();
                let mut take = |entry: &FigValue| {
                    let Some(guids) = entry.get("guidPath").and_then(|p| p.get_array("guids"))
                    else {
                        return;
                    };
                    if guids.len() != 1 {
                        return;
                    }
                    let Some(pk) = guids.first().and_then(guid_to_string) else {
                        return;
                    };
                    let lid = pk
                        .split(':')
                        .nth(1)
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let slot = per_sym.entry(pk).or_insert((lid, Vec::new(), false));
                    if let Some(c) = entry
                        .get("textData")
                        .or_else(|| entry.get("derivedTextData"))
                        .and_then(|t| t.get_str("characters"))
                    {
                        slot.1.push(c.to_string());
                        slot.2 = true;
                    }
                };
                if let Some(ov) = figma
                    .get("symbolData")
                    .and_then(|s| s.get_array("symbolOverrides"))
                {
                    for e in ov {
                        take(e);
                    }
                }
                if let Some(dv) = figma.get_array("derivedSymbolData") {
                    for e in dv {
                        take(e);
                    }
                }
            }
        }
        for c in &node.children {
            collect(c, pool);
        }
    }
    collect(root, &mut pool);

    // Geometry seeding: an instance whose single-segment entries carry
    // BOTH a size and a transform proves the shared pk -> node mapping
    // for its whole symbol — instances of the same SYMBOL share one
    // virtual-ID space, so a sibling with rich derived data resolves
    // pks that a sparsely-overridden instance could only walk-guess.
    // pk, lid, geometry-bearing entry, same-pk override entry (its
    // fill hint is the tie-breaker between geometry near-ties).
    type GeomEntry = (String, f64, FigValue, Option<FigValue>);
    type GeomSeeds = Vec<(String, Option<FigVec2>, Vec<GeomEntry>)>;
    let mut geom_seeds: GeomSeeds = Vec::new();

    fn collect_geom(node: &TreeNode, out: &mut GeomSeeds) {
        let figma = &node.figma;
        // Swapped instances (see `collect`) must not seed the base
        // component's cache — their geometry is the swapped-in one's.
        if figma.get_str("type") == Some("INSTANCE") && figma.get("overriddenSymbolID").is_none() {
            if let Some(sym_guid) = figma
                .get("symbolData")
                .and_then(|s| s.get("symbolID"))
                .and_then(|g| {
                    Some(format!(
                        "{}:{}",
                        g.get_f64("sessionID")? as u64,
                        g.get_f64("localID")? as u64
                    ))
                })
            {
                let single_pk = |entry: &FigValue| -> Option<String> {
                    let guids = entry.get("guidPath").and_then(|p| p.get_array("guids"))?;
                    if guids.len() != 1 {
                        return None;
                    }
                    guids.first().and_then(guid_to_string)
                };
                let overrides = figma
                    .get("symbolData")
                    .and_then(|s| s.get_array("symbolOverrides"));
                let override_for = |pk: &str| -> Option<FigValue> {
                    overrides?
                        .iter()
                        .find(|e| single_pk(e).as_deref() == Some(pk))
                        .cloned()
                };
                let mut entries: Vec<GeomEntry> = Vec::new();
                let mut take = |entry: &FigValue| {
                    let Some(pk) = single_pk(entry) else {
                        return;
                    };
                    let has_geom = entry.get("size").is_some()
                        && entry
                            .get("transform")
                            .map(|t| t.get_f64("m02").is_some() && t.get_f64("m12").is_some())
                            .unwrap_or(false);
                    if !has_geom || entries.iter().any(|(p, _, _, _)| *p == pk) {
                        return;
                    }
                    let lid = pk
                        .split(':')
                        .nth(1)
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let ov = override_for(&pk);
                    entries.push((pk, lid, entry.clone(), ov));
                };
                if let Some(ov) = overrides {
                    for e in ov {
                        take(e);
                    }
                }
                if let Some(dv) = figma.get_array("derivedSymbolData") {
                    for e in dv {
                        take(e);
                    }
                }
                if !entries.is_empty() {
                    let inst_size = figma.get("size").and_then(FigVec2::from_value);
                    out.push((sym_guid, inst_size, entries));
                }
            }
        }
        for c in &node.children {
            collect_geom(c, out);
        }
    }
    collect_geom(root, &mut geom_seeds);

    for (sym_guid, per_pk) in &pool {
        let Some(symbol) = symbol_tree.get(sym_guid) else {
            continue;
        };
        let mut flat: Vec<&TreeNode> = Vec::new();
        flatten_dfs(symbol, &mut flat);
        let candidates: Vec<&TreeNode> = flat[1..].to_vec();
        let base_lid = per_pk
            .values()
            .map(|(lid, _, _)| *lid)
            .fold(f64::INFINITY, f64::min);
        let mut entries: Vec<fingerprint::PooledEntry> = per_pk
            .iter()
            .map(|(pk, (lid, chars, demands))| fingerprint::PooledEntry {
                pk: pk.clone(),
                rel_idx: lid - base_lid,
                char_values: chars.clone(),
                demands_text: *demands,
            })
            .collect();
        entries.sort_by(|a, b| a.pk.cmp(&b.pk));
        for (pk, ng) in fingerprint::assign_pooled(&entries, &candidates) {
            // First seeding wins — the clipboard path seeds the whole
            // tree first and then per top-node; a later, narrower pool
            // must not overwrite the global pooled assignment.
            cache.entry(format!("{sym_guid}|{pk}")).or_insert(ng);
        }
    }

    for (sym_guid, inst_size, entries) in &geom_seeds {
        let Some(symbol) = symbol_tree.get(sym_guid) else {
            continue;
        };
        let mut flat: Vec<&TreeNode> = Vec::new();
        flatten_dfs(symbol, &mut flat);
        if flat.len() < 2 {
            continue;
        }
        let candidates: Vec<&TreeNode> = flat[1..].to_vec();
        let base = entries
            .iter()
            .map(|(_, lid, _, _)| *lid)
            .fold(f64::INFINITY, f64::min);
        let ventries: Vec<fingerprint::VirtualEntry> = entries
            .iter()
            .map(|(pk, lid, e, ov)| fingerprint::VirtualEntry {
                pk: pk.clone(),
                rel_idx: lid - base,
                derived: Some(e),
                overrides: ov.as_ref(),
            })
            .collect();
        let ratios = match (
            *inst_size,
            symbol.figma.get("size").and_then(FigVec2::from_value),
        ) {
            (Some(i), Some(sym)) if sym.x > 0.0 && sym.y > 0.0 => (i.x / sym.x, i.y / sym.y),
            _ => (1.0, 1.0),
        };
        let Some(assigned) = fingerprint::assign(&ventries, &candidates, ratios) else {
            continue;
        };
        for (pk, ng) in assigned {
            // Pooled text pins (above) win collisions — they carry
            // cross-instance evidence; geometry is per-instance.
            cache.entry(format!("{sym_guid}|{pk}")).or_insert(ng);
        }
    }
}

/// Clone the symbol children rescaled to the instance size: the
/// no-override fast path, also the fallback when a guessed mapping
/// fails the confidence gate.
pub(super) fn rescale_only(
    symbol_node: &TreeNode,
    instance_size: Option<FigVec2>,
) -> Vec<TreeNode> {
    if let (Some(size), Some(Some(sym_size))) = (
        instance_size,
        symbol_node.figma.get("size").map(FigVec2::from_value),
    ) {
        if sym_size.x != 0.0 && sym_size.y != 0.0 {
            let sx = size.x / sym_size.x;
            let sy = size.y / sym_size.y;
            return scale_tree_children(&symbol_node.children, sx, sy);
        }
    }
    symbol_node.children.clone()
}

/// Whether a walk-order-guessed derived-to-node mapping contradicts
/// itself. A derived entry's size should sit near either the mapped
/// node's authored size or that size stretched by the instance/symbol
/// ratio; an entry far from BOTH (>20 px and >50% off in a dimension)
/// is a severe mismatch. A severe MAJORITY (and at least two entries)
/// means the guess keyed data to the wrong nodes; a minority is just
/// legitimate per-node stretching. The symbol root is excluded: its
/// derived data never applies to the returned children.
pub(super) fn guessed_mapping_is_implausible(
    node_derived: &HashMap<String, FigValue>,
    flat_symbol: &[&TreeNode],
    instance_size: Option<FigVec2>,
    symbol_node: &TreeNode,
) -> bool {
    let (sx, sy) = match (
        instance_size,
        symbol_node.figma.get("size").map(FigVec2::from_value),
    ) {
        (Some(inst), Some(Some(sym))) if sym.x > 0.0 && sym.y > 0.0 => {
            (inst.x / sym.x, inst.y / sym.y)
        }
        _ => (1.0, 1.0),
    };
    let root_key = symbol_node
        .figma
        .get("guid")
        .and_then(guid_to_string)
        .unwrap_or_default();
    let mut node_size: HashMap<String, FigVec2> = HashMap::new();
    for n in flat_symbol {
        if let (Some(k), Some(sz)) = (
            n.figma.get("guid").and_then(guid_to_string),
            n.figma.get("size").and_then(FigVec2::from_value),
        ) {
            node_size.insert(k, sz);
        }
    }
    fn far(derived: f64, authored: f64, scaled: f64) -> bool {
        fn off(a: f64, b: f64) -> bool {
            (a - b).abs() > 20.0 && (a - b).abs() > 0.5 * b.max(1.0)
        }
        off(derived, authored) && off(derived, scaled)
    }
    let mut severe = 0usize;
    let mut comparable = 0usize;
    for (guid_key, d) in node_derived {
        if *guid_key == root_key {
            continue;
        }
        let Some(dsz) = d.get("size").and_then(FigVec2::from_value) else {
            continue;
        };
        let Some(nsz) = node_size.get(guid_key) else {
            continue;
        };
        comparable += 1;
        if far(dsz.x, nsz.x, nsz.x * sx) || far(dsz.y, nsz.y, nsz.y * sy) {
            severe += 1;
        }
    }
    severe >= 2 && severe * 2 > comparable
}
