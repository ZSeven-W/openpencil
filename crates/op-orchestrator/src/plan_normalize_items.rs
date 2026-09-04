//! DS P2-a experiment (item ②) — planning-layer "repeated item family
//! bundling", gated to the deepseek model family.
//!
//! Measured on deepseek-v4-pro card boards: a "五条法则" plan fan out into
//! five subtasks ("法则 01".."法则 05"), and the model re-invented the item
//! structure per subtask — five items, five structures. Merging the family
//! into ONE subtask makes the model emit the whole list from one prompt, so
//! the ITEM TEMPLATE teaching (the public `cards` contract plus the DS
//! `card-item-template` overlay) can actually hold across the copies.
//!
//! Strategic line: output contracts belong in the public corpus, model
//! behaviour adaptation belongs in the DS experiment field. This bundling is
//! a MODEL-behaviour adaptation — it rewrites the plan, not the design
//! contract — so it lives here, gated to the deepseek family, and graduates
//! (gate removed) only after ab validation shows glm/kimi do not regress
//! with the same bundling. The gate admits nothing when `model_id` is empty,
//! so every caller that does not know its model keeps today's plan
//! byte-for-byte.

use crate::plan::{OrchestratorPlan, Subtask};

/// Minimum members for one family (the DS P2-a gate threshold).
const FAMILY_MIN_MEMBERS: usize = 3;

/// The family stem of a subtask: its label with trailing digits (plus the
/// whitespace before them) stripped — "法则 01" → "法则", "Item 2" → "Item".
/// When the label leaves nothing (a pure ordinal), fall back to the id stem.
/// Mirrors `orchestration_self_check::value_name_stem`'s digit-strip rule so
/// the planning layer and the post-hoc echo layer agree on what a family is.
fn strip_trailing_digits(s: &str) -> &str {
    s.trim_end()
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .trim_end()
}

/// The family stem of one subtask (label first, id as fallback).
fn subtask_stem(st: &Subtask) -> String {
    let label_stem = strip_trailing_digits(&st.label);
    if label_stem.is_empty() {
        strip_trailing_digits(&st.id).to_string()
    } else {
        label_stem.to_string()
    }
}

/// Merge repeated item-family subtasks into one subtask — DS-gated, in place.
///
/// Groups subtasks by `(stem, screen)`: the stem comes from
/// [`subtask_stem`], and the `screen` label is part of the key because the
/// multi-screen fanout owns the screen dimension — subtasks tagged with
/// DIFFERENT screens are never merged (a deck's slides each carry their own
/// screen, so slide families stay per-slide; see `plan_fallback_deck`).
/// A group of ≥ [`FAMILY_MIN_MEMBERS`] members collapses into its first
/// member: the description concatenates the members' labels in order,
/// annotated with the member count; every other field is the first member's
/// verbatim.
pub(super) fn bundle_repeated_item_families(plan: &mut OrchestratorPlan, model_id: &str) {
    // The gate: only the deepseek family gets this adaptation while the
    // experiment runs. An empty model id (the default on paths that do not
    // know their model) never enables it.
    if !op_ai_skills::resolver::model_id_matches_family(model_id, "deepseek") {
        return;
    }

    // Family key per subtask — first-seen order is what keeps the merged
    // member at the position the family originally started from.
    let mut order: Vec<(String, Option<String>)> = Vec::new();
    let mut groups: std::collections::HashMap<(String, Option<String>), Vec<usize>> =
        std::collections::HashMap::new();
    for (index, st) in plan.subtasks.iter().enumerate() {
        let stem = subtask_stem(st);
        if stem.is_empty() {
            continue; // no stem, no family.
        }
        let key = (stem, st.screen.clone());
        match groups.get_mut(&key) {
            Some(indices) => indices.push(index),
            None => {
                order.push(key.clone());
                groups.insert(key, vec![index]);
            }
        }
    }

    let mut skip: Vec<bool> = vec![false; plan.subtasks.len()];
    let mut merged_labels: std::collections::HashMap<usize, String> =
        std::collections::HashMap::new();
    for key in &order {
        let indices = &groups[key];
        if indices.len() < FAMILY_MIN_MEMBERS {
            continue;
        }
        let first = indices[0];
        let labels: Vec<&str> = indices
            .iter()
            .map(|&i| plan.subtasks[i].label.as_str())
            .collect();
        for &member in &indices[1..] {
            skip[member] = true;
        }
        merged_labels.insert(
            first,
            format!(
                "{} ({} items: {})",
                plan.subtasks[first].label,
                indices.len(),
                labels.join("、")
            ),
        );
    }

    if merged_labels.is_empty() {
        return;
    }

    let mut out: Vec<Subtask> = Vec::with_capacity(plan.subtasks.len());
    for (index, st) in plan.subtasks.iter().enumerate() {
        if skip[index] {
            continue;
        }
        let mut merged = st.clone();
        if let Some(label) = merged_labels.get(&index) {
            merged.label = label.clone();
        }
        out.push(merged);
    }
    plan.subtasks = out;
}
