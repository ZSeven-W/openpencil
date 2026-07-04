//! Component-swap stale-derived filtering — when an
//! `overriddenSymbolID` swaps a nested instance's component, the raw
//! `derivedSymbolData` still carries the pre-swap component's entries
//! alongside the swapped-in one. This module drops the stale cluster
//! so it can\'t hijack the fingerprint mapping onto the new subtree.

use super::flatten_dfs;
use crate::figma_types::FigVec2;
use crate::kiwi::FigValue;
use crate::tree::TreeNode;

/// Drop the STALE pre-swap component's derived cluster when an
/// `overriddenSymbolID` swap has left it in the array. Figma keeps the
/// base component's derived (the icon before the swap) AND the
/// swapped-in component's derived; when the two frames happen to be
/// the same size the fingerprint can't tell them apart and the stale
/// (earlier-listed) cluster hijacks the mapping. Single-segment
/// entries are split into contiguous localID clusters and each is
/// scored by how well it fits the swapped subtree's node sizes; only
/// the best-fitting cluster's single-segment entries survive.
/// Multi-segment (nested) entries are always kept. A no-op when there
/// is one cluster or the best doesn't clearly beat the rest.
pub(crate) fn filter_swap_stale_derived(
    derived: &[FigValue],
    symbol_node: &TreeNode,
    instance_size: Option<FigVec2>,
) -> Vec<FigValue> {
    // Candidate node sizes (subtree minus root) + instance ratio.
    let mut flat: Vec<&TreeNode> = Vec::new();
    flatten_dfs(symbol_node, &mut flat);
    let candidates: Vec<FigVec2> = flat[1..]
        .iter()
        .filter_map(|n| n.figma.get("size").and_then(FigVec2::from_value))
        .collect();
    if candidates.is_empty() {
        return derived.to_vec();
    }
    let (rx, ry) = match (
        instance_size,
        symbol_node.figma.get("size").and_then(FigVec2::from_value),
    ) {
        (Some(i), Some(s)) if s.x > 0.0 && s.y > 0.0 => (i.x / s.x, i.y / s.y),
        _ => (1.0, 1.0),
    };

    // Single-segment entries with a localID + size; everything else is
    // kept verbatim.
    struct SingleEntry {
        idx: usize,
        lid: u32,
        size: FigVec2,
    }
    let mut singles: Vec<SingleEntry> = Vec::new();
    for (idx, e) in derived.iter().enumerate() {
        let guids = e.get("guidPath").and_then(|p| p.get_array("guids"));
        let Some(guids) = guids else { continue };
        if guids.len() != 1 {
            continue;
        }
        let Some(lid) = guids.first().and_then(|g| g.get_f64("localID")) else {
            continue;
        };
        let Some(size) = e.get("size").and_then(FigVec2::from_value) else {
            continue;
        };
        singles.push(SingleEntry {
            idx,
            lid: lid as u32,
            size,
        });
    }
    if singles.len() < 2 {
        return derived.to_vec();
    }

    // Cluster by contiguous localID (gap > 16 starts a new cluster).
    singles.sort_by_key(|s| s.lid);
    let mut clusters: Vec<Vec<&SingleEntry>> = Vec::new();
    for s in &singles {
        match clusters.last_mut() {
            Some(last) if s.lid - last.last().unwrap().lid <= 16 => last.push(s),
            _ => clusters.push(vec![s]),
        }
    }
    if clusters.len() < 2 {
        return derived.to_vec();
    }

    // Fit score: mean over the cluster of the best near-match (0..1,
    // 1 = exact) to any candidate node, dual-baseline (authored or
    // ratio-scaled) like the fingerprint scorer.
    let fit = |cluster: &[&SingleEntry]| -> f64 {
        let mut total = 0.0;
        for s in cluster {
            let best = candidates
                .iter()
                .map(|c| {
                    let dx = (s.size.x - c.x).abs().min((s.size.x - c.x * rx).abs());
                    let dy = (s.size.y - c.y).abs().min((s.size.y - c.y * ry).abs());
                    dx + dy
                })
                .fold(f64::INFINITY, f64::min);
            total += (1.0 - best / 4.0).max(0.0);
        }
        total / cluster.len() as f64
    };
    let mut scored: Vec<(f64, usize)> = clusters
        .iter()
        .enumerate()
        .map(|(i, c)| (fit(c), i))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let (best_score, best_i) = scored[0];
    let (second_score, _) = scored[1];
    // Only prune when the winner clearly fits better — a swapped-in
    // cluster fits nearly perfectly while the stale one mostly doesn't.
    if best_score < 0.6 || best_score - second_score < 0.2 {
        return derived.to_vec();
    }
    let keep_lids: std::collections::HashSet<u32> =
        clusters[best_i].iter().map(|s| s.lid).collect();
    let drop_idx: std::collections::HashSet<usize> = singles
        .iter()
        .filter(|s| !keep_lids.contains(&s.lid))
        .map(|s| s.idx)
        .collect();
    derived
        .iter()
        .enumerate()
        .filter(|(i, _)| !drop_idx.contains(i))
        .map(|(_, e)| e.clone())
        .collect()
}
