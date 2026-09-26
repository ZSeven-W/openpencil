//! Structural drift echo (DS P1.5, INTENT-class).
//!
//! ≥3 sibling Frame sections under one parent that share a name stem (digits
//! stripped) or a list-shaped role run, whose recursive structure
//! signatures disagree — measured 0815-08-15 on the v4-pro card where five
//! "法则" items shipped five different internal structures. Structure is
//! INTENT, so this check never auto-fixes. Its issue carries
//! [`SelfCheckSeverity::Intent`]: the retry ladder's early attempts reject
//! on it so the retry nudge tells the model to re-emit the family from ONE
//! template, but the FINAL attempt (and the salvage pass) accepts the
//! content and only logs the finding — a structure opinion must never
//! delete a whole section (GLM-5.3-Flash arena 0926c lost a kanban board
//! and a pricing section that way). A group where >= 2/3 of the members
//! share one signature is exempt — a deliberate hero first item is not
//! drift (the same hero exemption `cleanup_equalize_siblings` votes under).

use std::collections::BTreeMap;

use super::*;

/// Minimum members for a drift group.
const DRIFT_MIN_MEMBERS: usize = 3;
/// Hero exemption: a modal structure held by >= 2/3 of the group.
const DRIFT_MAJORITY_NUM: usize = 2;
const DRIFT_MAJORITY_DEN: usize = 3;

/// One section-structure-drift finding plus the ids of the drifting
/// siblings — the payload `finalize_design`'s summary surfaces as an
/// advisory (DS P2-a item ③, echo-only: report, never auto-fix).
pub(super) struct DriftHit {
    pub(super) node_ids: Vec<String>,
    pub(super) message: String,
}

/// The drift message for `node`'s children, or `None` when no group drifts.
pub(super) fn sibling_structure_drift(node: &Value) -> Option<String> {
    sibling_structure_drift_hit(node).map(|hit| hit.message)
}

/// [`sibling_structure_drift`] with the drifting siblings' ids attached.
pub(super) fn sibling_structure_drift_hit(node: &Value) -> Option<DriftHit> {
    let children = children(node)?;
    let members: Vec<&Value> = children
        .iter()
        .filter(|child| string_prop(child, "type") == Some("frame"))
        .filter(|child| child.get("visible").and_then(Value::as_bool) != Some(false))
        .collect();
    if members.len() < DRIFT_MIN_MEMBERS {
        return None;
    }

    let mut groups: Vec<Vec<&Value>> = Vec::new();
    // Name-stem groups: "法则 01" / "法则 02" — digits stripped.
    let mut by_stem: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for member in &members {
        let stem = value_name_stem(string_prop(member, "name").unwrap_or(""));
        if !stem.is_empty() {
            by_stem.entry(stem).or_default().push(member);
        }
    }
    groups.extend(
        by_stem
            .into_values()
            .filter(|group| group.len() >= DRIFT_MIN_MEMBERS),
    );
    groups.extend(role_list_runs(&members));

    for group in groups {
        let signatures: Vec<String> = group
            .iter()
            .map(|member| value_structure_signature(member))
            .collect();
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for signature in &signatures {
            *counts.entry(signature.as_str()).or_default() += 1;
        }
        let distinct = counts.len();
        if distinct <= 1 {
            continue; // isomorphic group — nothing to say.
        }
        let modal = counts.values().copied().max().unwrap_or(0);
        if modal * DRIFT_MAJORITY_DEN >= group.len() * DRIFT_MAJORITY_NUM {
            continue; // hero exemption: the family norm holds, the odd one out is deliberate.
        }
        let node_ids: Vec<String> = group
            .iter()
            .filter_map(|member| string_prop(member, "id").map(str::to_string))
            .collect();
        let names: Vec<String> = group
            .iter()
            .map(|member| string_prop(member, "name").unwrap_or("?").to_string())
            .collect();
        return Some(DriftHit {
            node_ids,
            message: format!(
                "the {} sibling frame sections {} share one family but carry {distinct} different \
                 subtree structures; unify them on ONE structure template — same nesting, same \
                 children, same name pattern, only the content differs",
                group.len(),
                names.join(", ")
            ),
        });
    }
    None
}

/// Role groups — only where the role marks ONE LIST of like items.
///
/// A shared role alone is not a family: role inference stamps `card` on
/// every frame whose name contains the word, so `pricing-card-pro-header`,
/// `-price` and `-cta` (three different PARTS of one card) all carried
/// `role: "card"` and were rejected as a drifting family. Two guards, each
/// only narrowing the old role path (the name-stem path is untouched):
///
/// 1. **The role must say something the name does not.** A member whose
///    role is exactly what [`crate::role_infer::infer_role_from_frame_name`]
///    reads off its own name contributes no independent family signal —
///    names already group through their stem. This is what separates the
///    pricing parts (inferred `card`) from the 0815 lesion (authored
///    `section-rule` on names `法则条目` / `Section Rule` / `Rule 03 Row`).
/// 2. **The members must be list-shaped:** a run of >= 3 CONSECUTIVE frame
///    siblings with the same role and the same `layout`. Items of one list
///    sit next to each other and flow the same way; a header / price row /
///    CTA that merely share an authored generic role rarely do both.
fn role_list_runs<'a>(members: &[&'a Value]) -> Vec<Vec<&'a Value>> {
    let key = |member: &'a Value| -> Option<(&'a str, Option<&'a str>)> {
        let role = string_prop(member, "role").filter(|role| !role.is_empty())?;
        let name = string_prop(member, "name").unwrap_or("");
        if crate::role_infer::infer_role_from_frame_name(name) == Some(role) {
            return None;
        }
        Some((role, string_prop(member, "layout")))
    };
    let mut runs: Vec<Vec<&'a Value>> = Vec::new();
    let mut current: Vec<&'a Value> = Vec::new();
    let mut current_key = None;
    for member in members {
        let member_key = key(member);
        if member_key.is_none() || member_key != current_key {
            if current.len() >= DRIFT_MIN_MEMBERS {
                runs.push(std::mem::take(&mut current));
            }
            current.clear();
            current_key = member_key;
        }
        if current_key.is_some() {
            current.push(member);
        }
    }
    if current.len() >= DRIFT_MIN_MEMBERS {
        runs.push(current);
    }
    runs
}

// ── Echo-only advisories for external finalize callers (DS P2-a item ③) ──────
//
// The pre-insertion self-check rejects a drifting subtask on the retry
// ladder's early attempts. The `finalize_design` MCP tool cannot reject — it
// runs AFTER the fact — so it reuses this same detector over the final
// document and surfaces hits as ADVISORIES in its summary JSON: reported,
// never repaired, never counted in the repair tally, never applied to the
// document. The model-in-the-loop fixes them through batch_design /
// update_node and finalizes again (the dsh chain's self-repair interface
// surface; see the finalize_design schema description).
//
// Visibility note: the parent module is `pub mod` (was `pub(crate)`) so
// `op-host-services` — the crate that owns the MCP tool — can reach this
// function. The dependency direction stays services → orchestrator, the
// same direction `loop_finalize` / `cleanup` already use.

/// One section-structure-drift advisory for the `finalize_design` summary:
/// the drifting sibling ids plus the explanatory message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionStructureDriftAdvisory {
    pub code: &'static str,
    pub node_ids: Vec<String>,
    pub message: String,
}

/// Run the sibling-structure-drift detector over every parent node of the
/// `nodes` forest (the document's active-page children) and collect the
/// hits as advisories. Read-only: the document is never modified here.
pub fn collect_section_structure_drift(nodes: &[PenNode]) -> Vec<SectionStructureDriftAdvisory> {
    let value = serde_json::to_value(nodes).unwrap_or(Value::Null);
    let mut advisories = Vec::new();
    collect_structure_drift(&value, &mut advisories);
    advisories
}

fn collect_structure_drift(value: &Value, advisories: &mut Vec<SectionStructureDriftAdvisory>) {
    match value {
        Value::Array(nodes) => {
            for node in nodes {
                visit_structure_drift(node, advisories);
            }
        }
        Value::Object(_) => visit_structure_drift(value, advisories),
        _ => {}
    }
}

/// Check `node`'s children as one sibling family, then recurse — the same
/// walk shape as `check_node` (the page root itself is not a family).
fn visit_structure_drift(node: &Value, advisories: &mut Vec<SectionStructureDriftAdvisory>) {
    if let Some(hit) = sibling_structure_drift_hit(node) {
        advisories.push(SectionStructureDriftAdvisory {
            code: "section-structure-drift",
            node_ids: hit.node_ids,
            message: hit.message,
        });
    }
    if let Some(children) = children(node) {
        for child in children {
            visit_structure_drift(child, advisories);
        }
    }
}

/// The name with trailing digits (and the whitespace before them) stripped:
/// "Item 01" → "Item", "Item02" → "Item". Mirrors the equalize pass's
/// [`crate::cleanup::cleanup_equalize_siblings`] name-stem rule.
fn value_name_stem(name: &str) -> &str {
    let trimmed = name.trim_end();
    trimmed
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .trim_end()
}

/// Recursive structure signature: `type(child child …)`, where a run of
/// CONSECUTIVE identical child signatures collapses to one. List length is
/// content, not structure — a kanban column holding 3 identical task cards
/// and one holding 5 share a signature, while a card that grows an extra
/// badge (or reorders its parts) still reads as a different structure.
pub(super) fn value_structure_signature(node: &Value) -> String {
    let mut signature = string_prop(node, "type").unwrap_or("?").to_string();
    let kids = children(node).map(Vec::as_slice).unwrap_or(&[]);
    if kids.is_empty() {
        return signature;
    }
    signature.push('(');
    let mut previous: Option<String> = None;
    for child in kids {
        let child_signature = value_structure_signature(child);
        if previous.as_deref() == Some(child_signature.as_str()) {
            continue;
        }
        if previous.is_some() {
            signature.push(' ');
        }
        signature.push_str(&child_signature);
        previous = Some(child_signature);
    }
    signature.push(')');
    signature
}
