//! Foreign-session virtual entries — resolution by validated subtree
//! anchoring.
//!
//! An instance's override entries usually share one virtual session
//! (the base). Files with nested library components carry EXTRA
//! sessions whose numbering is anchored at some SUBTREE of the symbol
//! (that component's own history space). The anchor is not encoded, so
//! it is searched: every (node, self|children) walk is tried, and one
//! is trusted only when it is the UNIQUE walk under which every
//! text-demanding entry of the group lands on a TEXT node. Groups with
//! no such unique anchor keep the safe behavior — dropped.

use super::{local_id, walk_virtual, TreeNode};
use std::collections::HashMap;

/// One foreign-session group: pks sharing a non-base sessionID.
pub(super) struct ForeignGroup {
    pub session: u32,
    /// pk → whether the entry demands a TEXT node.
    pub pks: Vec<(String, bool)>,
}

/// Split walk-missed pks into per-session groups.
pub(super) fn group_by_session(pks: &[(String, bool)]) -> Vec<ForeignGroup> {
    let mut by_session: HashMap<u32, Vec<(String, bool)>> = HashMap::new();
    for (pk, demand) in pks {
        let Some(session) = pk.split(':').next().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        by_session
            .entry(session)
            .or_default()
            .push((pk.clone(), *demand));
    }
    let mut out: Vec<ForeignGroup> = by_session
        .into_iter()
        .map(|(session, pks)| ForeignGroup { session, pks })
        .collect();
    out.sort_by_key(|g| g.session);
    out
}

fn min_lid(group: &ForeignGroup) -> Option<u32> {
    group
        .pks
        .iter()
        .filter_map(|(pk, _)| pk.split(':').nth(1).and_then(|s| s.parse::<u32>().ok()))
        .min()
}

/// Resolve one foreign group to `pk → node guid`, or `None` when no
/// unique validated anchor exists. Requires ≥2 text-demanding entries
/// — a single hit is too weak to certify an anchor.
pub(super) fn resolve_group(
    group: &ForeignGroup,
    symbol_node: &TreeNode,
) -> Option<HashMap<String, String>> {
    let base = min_lid(group)?;
    let text_pks: Vec<&String> = group
        .pks
        .iter()
        .filter(|(_, demand)| *demand)
        .map(|(pk, _)| pk)
        .collect();
    if text_pks.len() < 2 {
        return None;
    }

    // Candidate anchors: every subtree node, in two modes — the walk
    // starting AT the node (self) and at its children.
    let mut anchors: Vec<&TreeNode> = Vec::new();
    fn collect<'a>(n: &'a TreeNode, out: &mut Vec<&'a TreeNode>) {
        out.push(n);
        for c in &n.children {
            collect(c, out);
        }
    }
    collect(symbol_node, &mut anchors);

    let is_text =
        |map: &HashMap<String, String>, pk: &String, by_guid: &HashMap<String, &TreeNode>| {
            map.get(pk)
                .and_then(|g| by_guid.get(g))
                .map(|n| n.figma.get_str("type") == Some("TEXT"))
        };
    let mut by_guid: HashMap<String, &TreeNode> = HashMap::new();
    for n in &anchors {
        if let Some(g) = n.figma.get("guid").and_then(crate::tree::guid_to_string) {
            by_guid.insert(g, n);
        }
    }

    let mut winner: Option<HashMap<String, String>> = None;
    for anchor in &anchors {
        for children_mode in [false, true] {
            let mut map: HashMap<String, String> = HashMap::new();
            let mut idx: u32 = 0;
            if children_mode {
                let mut sorted: Vec<&TreeNode> = anchor.children.iter().collect();
                sorted.sort_by_key(|n| local_id(n));
                for c in sorted {
                    walk_virtual(c, group.session, base, &mut idx, &mut map);
                }
            } else {
                walk_virtual(anchor, group.session, base, &mut idx, &mut map);
            }
            // Every text-demanding pk must map, and map to TEXT.
            let ok = text_pks
                .iter()
                .all(|pk| is_text(&map, pk, &by_guid) == Some(true));
            if !ok {
                continue;
            }
            if winner.is_some() {
                // Ambiguous — two walks both validate. Not trustable.
                return None;
            }
            winner = Some(map);
        }
    }
    winner
}
