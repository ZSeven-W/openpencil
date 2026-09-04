use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelfCheckSeverity {
    Fatal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfCheckIssue {
    pub code: &'static str,
    pub node_id: Option<String>,
    pub message: String,
    pub severity: SelfCheckSeverity,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SelfCheckReport {
    pub issues: Vec<SelfCheckIssue>,
}

impl SelfCheckReport {
    pub(crate) fn has_fatal(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == SelfCheckSeverity::Fatal)
    }

    pub(crate) fn failure_message(&self) -> String {
        self.issues
            .iter()
            .filter(|issue| issue.severity == SelfCheckSeverity::Fatal)
            .map(|issue| match issue.node_id.as_deref() {
                Some(id) if !id.is_empty() => {
                    format!("{} at {}: {}", issue.code, id, issue.message)
                }
                _ => format!("{}: {}", issue.code, issue.message),
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

pub(crate) fn check_generated_nodes(nodes: &[PenNode], canvas_width: f64) -> SelfCheckReport {
    let value = serde_json::to_value(nodes).unwrap_or(Value::Null);
    let mut report = check_value_forest(&value, canvas_width);
    for missing in op_design_lint::detect_missing_progress_rings(nodes) {
        report.issues.push(SelfCheckIssue {
            code: "missing-progress-ring",
            node_id: Some(missing.node_id),
            message: format!(
                "{} contains a numeric metric but no visible circle or arc; render the intended progress ring with authored ellipse/arc geometry or a painted circular frame",
                missing.node_name
            ),
            severity: SelfCheckSeverity::Fatal,
        });
    }
    report
}

pub(crate) fn check_value_forest(value: &Value, canvas_width: f64) -> SelfCheckReport {
    let mut report = SelfCheckReport::default();
    match value {
        Value::Array(nodes) => {
            for node in nodes {
                check_node(node, canvas_width, false, &mut report);
            }
        }
        Value::Object(_) => check_node(value, canvas_width, false, &mut report),
        _ => {}
    }
    report
}

pub fn auto_fix_fixable_issues(nodes: &mut [PenNode], canvas_width: f64) -> bool {
    let mut value = serde_json::to_value(&*nodes).unwrap_or(Value::Null);
    let radial_changed = crate::radial_repair::repair_authored_radial_stacks(&mut value);
    let layout_changed = auto_fix_value_forest(&mut value, canvas_width);
    if !radial_changed && !layout_changed {
        return false;
    }

    let Ok(fixed_nodes) = serde_json::from_value::<Vec<PenNode>>(value) else {
        return false;
    };
    if fixed_nodes.len() != nodes.len() {
        return false;
    }
    for (node, fixed_node) in nodes.iter_mut().zip(fixed_nodes) {
        *node = fixed_node;
    }
    true
}

fn check_node(node: &Value, canvas_width: f64, in_scroller: bool, report: &mut SelfCheckReport) {
    let in_scroller = in_scroller || is_clipping_horizontal_scroller(node);
    if crate::radial_repair::is_authored_radial_stack_unsafe(node) {
        report.issues.push(SelfCheckIssue {
            code: "radial-stack-not-concentric",
            node_id: string_prop(node, "id").map(str::to_string),
            message: "progress-ring track, progress arc, and measurable centre content must share a fixed near-square layout:none wrapper with explicit concentric coordinates and front-to-back child order: centre, progress, track".into(),
            severity: SelfCheckSeverity::Fatal,
        });
    }
    if !in_scroller && is_mobile_product_row_overflow(node, canvas_width) {
        report.issues.push(SelfCheckIssue {
            code: "mobile-product-row-overflow",
            node_id: string_prop(node, "id").map(str::to_string),
            message:
                "fixed-width product cards exceed the mobile content rail; use two fill_container cards with gap 12"
                    .into(),
            severity: SelfCheckSeverity::Fatal,
        });
    }
    if is_mobile_category_row_loose_spacing(node, canvas_width) {
        report.issues.push(SelfCheckIssue {
            code: "mobile-category-row-loose-spacing",
            node_id: string_prop(node, "id").map(str::to_string),
            message:
                "mobile category rows must use start alignment, gap 12, fit_content height, and no wide fixed row"
                    .into(),
            severity: SelfCheckSeverity::Fatal,
        });
    }
    if is_mobile_featured_card_split_badly(node, canvas_width) {
        report.issues.push(SelfCheckIssue {
            code: "mobile-featured-card-bad-split",
            node_id: string_prop(node, "id").map(str::to_string),
            message:
                "mobile featured food cards must not leave a blank half beside the image; use an image-top product card or a deliberate 50/50 promo banner with compact action"
                    .into(),
            severity: SelfCheckSeverity::Fatal,
        });
    }
    if let Some(message) = sibling_structure_drift(node) {
        report.issues.push(SelfCheckIssue {
            code: "section-structure-drift",
            node_id: string_prop(node, "id").map(str::to_string),
            message,
            severity: SelfCheckSeverity::Fatal,
        });
    }

    if let Some(children) = children(node) {
        for child in children {
            check_node(child, canvas_width, in_scroller, report);
        }
    }
}

fn auto_fix_value_forest(value: &mut Value, canvas_width: f64) -> bool {
    match value {
        Value::Array(nodes) => {
            let mut changed = false;
            for node in nodes {
                changed |= auto_fix_node(node, canvas_width, false);
            }
            changed
        }
        Value::Object(_) => auto_fix_node(value, canvas_width, false),
        _ => false,
    }
}

fn auto_fix_node(node: &mut Value, canvas_width: f64, in_scroller: bool) -> bool {
    let mut changed = false;
    let in_scroller = in_scroller || is_clipping_horizontal_scroller(node);
    if !in_scroller && is_mobile_product_row_overflow(node, canvas_width) {
        changed |= auto_fix_product_row_overflow(node);
    }

    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            changed |= auto_fix_node(child, canvas_width, in_scroller);
        }
    }
    changed
}

fn auto_fix_product_row_overflow(node: &mut Value) -> bool {
    let mut changed = false;
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            if is_product_card_child(child) && numeric_prop(child, "width").is_some() {
                child["width"] = Value::String("fill_container".into());
                changed = true;
            }
        }
    }

    if numeric_prop(node, "gap")
        .map(|gap| gap > 12.0)
        .unwrap_or(true)
    {
        node["gap"] = Value::from(12);
        changed = true;
    }
    changed
}

fn is_clipping_horizontal_scroller(node: &Value) -> bool {
    string_prop(node, "layout") == Some("horizontal")
        && node.get("clipContent").and_then(Value::as_bool) == Some(true)
}

fn is_mobile_product_row_overflow(node: &Value, canvas_width: f64) -> bool {
    if canvas_width > 480.0
        || string_prop(node, "type") != Some("frame")
        || string_prop(node, "layout") != Some("horizontal")
    {
        return false;
    }
    if node.get("clipContent").and_then(Value::as_bool) == Some(true) {
        return false;
    }
    let Some(children) = children(node) else {
        return false;
    };
    if children.len() < 2 || !children.iter().all(is_product_card_child) {
        return false;
    }

    let fixed_widths: Vec<f64> = children
        .iter()
        .filter_map(|child| numeric_prop(child, "width"))
        .collect();
    if fixed_widths.len() < 2 {
        return false;
    }
    let gap = numeric_prop(node, "gap").unwrap_or(0.0);
    let total = fixed_widths.iter().sum::<f64>() + gap * (children.len().saturating_sub(1) as f64);
    total > available_row_width(node, canvas_width)
}

fn is_mobile_category_row_loose_spacing(node: &Value, canvas_width: f64) -> bool {
    if canvas_width > 480.0
        || string_prop(node, "type") != Some("frame")
        || string_prop(node, "layout") != Some("horizontal")
    {
        return false;
    }
    let Some(children) = children(node) else {
        return false;
    };
    if children.len() < 2 || !children.iter().all(is_category_item_child) {
        return false;
    }

    // `space_between` / `space_around` are NO LONGER flagged: spreading a small
    // chip set across the row is the desired distribution (the user's
    // "撑不满就把间距放大一点"). Only genuinely broken spacing is fatal — a huge
    // literal gap, a row wider than the canvas, or an over-tall row.
    numeric_prop(node, "gap")
        .map(|gap| gap > 48.0)
        .unwrap_or(false)
        || numeric_prop(node, "width")
            .map(|width| width > canvas_width)
            .unwrap_or(false)
        || numeric_prop(node, "height")
            .map(|height| height > 120.0)
            .unwrap_or(false)
}

fn is_mobile_featured_card_split_badly(node: &Value, canvas_width: f64) -> bool {
    if canvas_width > 480.0
        || string_prop(node, "type") != Some("frame")
        || string_prop(node, "layout") != Some("horizontal")
        || !looks_like_featured_food_card(node)
    {
        return false;
    }
    let Some(children) = children(node) else {
        return false;
    };
    if children.len() >= 2 && children.iter().all(is_product_card_child) {
        return false;
    }
    if children.len() < 2 || !has_descendant_type(node, "image") || !has_text_descendant(node) {
        return false;
    }

    let Some(card_width) = effective_node_width(node, canvas_width) else {
        return false;
    };
    let content_width = (card_width - horizontal_padding(node)).max(0.0);
    if content_width <= 0.0 {
        return false;
    }

    let gap = numeric_prop(node, "gap").unwrap_or(0.0);
    let fixed_child_total = children
        .iter()
        .filter_map(|child| numeric_prop(child, "width"))
        .sum::<f64>()
        + gap * (children.len().saturating_sub(1) as f64);
    let image_width_ratio = largest_descendant_image_width(node) / content_width;

    image_width_ratio < 0.45
        || (fixed_child_total > 0.0 && fixed_child_total < content_width * 0.82)
        || has_oversized_square_action(node)
}

fn looks_like_featured_food_card(node: &Value) -> bool {
    if !is_product_card_child(node) {
        return false;
    }
    let label = semantic_label(node);
    contains_any(
        &label,
        &[
            "featured",
            "feature",
            "special",
            "popular",
            "hero",
            "dish",
            "menu",
            "product",
            "truffle",
            "tagliatelle",
            "dumpling",
            "饺子",
            "特色",
            "推荐",
            "热门",
            "菜品",
        ],
    )
}

fn available_row_width(node: &Value, canvas_width: f64) -> f64 {
    let nominal = numeric_prop(node, "width")
        .filter(|width| *width > 0.0 && *width <= canvas_width)
        .unwrap_or_else(|| (canvas_width - 40.0).max(0.0));
    (nominal - horizontal_padding(node)).max(0.0)
}

fn effective_node_width(node: &Value, canvas_width: f64) -> Option<f64> {
    match node.get("width") {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) if s == "fill_container" => Some((canvas_width - 40.0).max(0.0)),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn horizontal_padding(node: &Value) -> f64 {
    match node.get("padding") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) * 2.0,
        Some(Value::Array(values)) if values.len() == 2 => {
            values.get(1).and_then(Value::as_f64).unwrap_or(0.0) * 2.0
        }
        Some(Value::Array(values)) if values.len() >= 4 => {
            values.get(1).and_then(Value::as_f64).unwrap_or(0.0)
                + values.get(3).and_then(Value::as_f64).unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

fn is_product_card_child(node: &Value) -> bool {
    if string_prop(node, "type") != Some("frame") {
        return false;
    }
    if matches!(
        string_prop(node, "role"),
        Some("card")
            | Some("image-card")
            | Some("product-card")
            | Some("restaurant-card")
            | Some("menu-card")
            | Some("feature-card")
    ) {
        return true;
    }

    let label = semantic_label(node);
    [
        "card",
        "product",
        "restaurant",
        "popular",
        "dish",
        "menu",
        "nearby",
        "餐厅",
        "美食",
        "热门",
        "菜品",
    ]
    .iter()
    .any(|needle| label.contains(needle))
        || (has_descendant_type(node, "image") && has_text_descendant(node))
}

fn is_category_item_child(node: &Value) -> bool {
    if string_prop(node, "type") != Some("frame") {
        return false;
    }
    matches!(
        string_prop(node, "role"),
        Some("chip") | Some("tag") | Some("pill") | Some("button")
    ) || {
        let label = semantic_label(node);
        label.contains("chip") || label.contains("category") || label.contains("类别")
    } || (has_descendant_type(node, "icon_font") && has_text_descendant(node))
}

fn has_descendant_type(node: &Value, type_name: &str) -> bool {
    string_prop(node, "type") == Some(type_name)
        || children(node)
            .map(|children| {
                children
                    .iter()
                    .any(|child| has_descendant_type(child, type_name))
            })
            .unwrap_or(false)
}

fn has_text_descendant(node: &Value) -> bool {
    (string_prop(node, "type") == Some("text")
        && string_prop(node, "content")
            .map(|content| !content.trim().is_empty())
            .unwrap_or(false))
        || children(node)
            .map(|children| children.iter().any(has_text_descendant))
            .unwrap_or(false)
}

fn largest_descendant_image_width(node: &Value) -> f64 {
    let own = if string_prop(node, "type") == Some("image") {
        numeric_prop(node, "width").unwrap_or(0.0)
    } else {
        0.0
    };
    let child = children(node)
        .map(|children| {
            children
                .iter()
                .map(largest_descendant_image_width)
                .fold(0.0, f64::max)
        })
        .unwrap_or(0.0);
    own.max(child)
}

fn has_oversized_square_action(node: &Value) -> bool {
    let label = semantic_label(node);
    let looks_action = contains_any(
        &label,
        &["button", "action", "add", "plus", "cta", "加入", "添加"],
    );
    let size = numeric_prop(node, "width").zip(numeric_prop(node, "height"));
    if looks_action
        && size
            .map(|(w, h)| w >= 56.0 && h >= 56.0 && (w - h).abs() <= 8.0)
            .unwrap_or(false)
    {
        return true;
    }
    children(node)
        .map(|children| children.iter().any(has_oversized_square_action))
        .unwrap_or(false)
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn semantic_label(node: &Value) -> String {
    [
        "id",
        "name",
        "role",
        "content",
        "placeholder",
        "value",
        "iconFontName",
    ]
    .iter()
    .filter_map(|key| string_prop(node, key))
    .collect::<Vec<_>>()
    .join(" ")
    .to_lowercase()
}

fn children(node: &Value) -> Option<&Vec<Value>> {
    node.get("children").and_then(Value::as_array)
}

// ── Structural drift echo (DS P1.5, intent-class — echo only) ─────────────────
//
// ≥3 sibling Frame sections under one parent that share a name stem (digits
// stripped) or a role, whose subtree kind-sequences disagree — measured
// 0815-08-15 on the v4-pro card where five "法则" items shipped five
// different internal structures. Structure is INTENT, so this check never
// auto-fixes: it is Fatal, so the existing self-check chain rejects the
// subtask and the retry nudge tells the model to re-emit the family from
// ONE template. A group where >= 2/3 of the members share one kind-sequence
// is exempt — a deliberate hero first item is not drift (the same hero
// exemption `cleanup_equalize_siblings` votes under).

/// Minimum members for a drift group.
const DRIFT_MIN_MEMBERS: usize = 3;
/// Hero exemption: a modal structure held by >= 2/3 of the group.
const DRIFT_MAJORITY_NUM: usize = 2;
const DRIFT_MAJORITY_DEN: usize = 3;

/// One section-structure-drift finding plus the ids of the drifting
/// siblings — the payload `finalize_design`'s summary surfaces as an
/// advisory (DS P2-a item ③, echo-only: report, never auto-fix).
struct DriftHit {
    node_ids: Vec<String>,
    message: String,
}

/// The drift message for `node`'s children, or `None` when no group drifts.
fn sibling_structure_drift(node: &Value) -> Option<String> {
    sibling_structure_drift_hit(node).map(|hit| hit.message)
}

/// [`sibling_structure_drift`] with the drifting siblings' ids attached.
fn sibling_structure_drift_hit(node: &Value) -> Option<DriftHit> {
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
    // Role groups: every member carries the same non-empty role.
    let mut by_role: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for member in &members {
        if let Some(role) = string_prop(member, "role") {
            if !role.is_empty() {
                by_role.entry(role).or_default().push(member);
            }
        }
    }
    groups.extend(
        by_role
            .into_values()
            .filter(|group| group.len() >= DRIFT_MIN_MEMBERS),
    );

    for group in groups {
        let sequences: Vec<String> = group
            .iter()
            .map(|member| value_kind_sequence(member))
            .collect();
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for sequence in &sequences {
            *counts.entry(sequence.as_str()).or_default() += 1;
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

// ── Echo-only advisories for external finalize callers (DS P2-a item ③) ──────
//
// The pre-insertion self-check rejects a drifting subtask (Fatal). The
// `finalize_design` MCP tool cannot reject — it runs AFTER the fact — so it
// reuses this same detector over the final document and surfaces hits as
// ADVISORIES in its summary JSON: reported, never repaired, never counted in
// the repair tally, never applied to the document. The model-in-the-loop
// fixes them through batch_design / update_node and finalizes again (the
// dsh chain's self-repair interface surface; see the finalize_design schema
// description).
//
// Visibility note: this module is `pub mod` (was `pub(crate)`) so
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
/// DFS pre-order kind-sequence over the JSON tree ("frame text image").
/// Two members with the same sequence share node types in the same
/// traversal positions.
fn value_kind_sequence(node: &Value) -> String {
    let mut sequence = String::new();
    push_value_kind_sequence(node, &mut sequence);
    sequence
}

fn push_value_kind_sequence(node: &Value, sequence: &mut String) {
    if !sequence.is_empty() {
        sequence.push(' ');
    }
    sequence.push_str(string_prop(node, "type").unwrap_or("?"));
    if let Some(children) = children(node) {
        for child in children {
            push_value_kind_sequence(child, sequence);
        }
    }
}

fn string_prop<'a>(node: &'a Value, key: &str) -> Option<&'a str> {
    node.get(key).and_then(Value::as_str)
}

fn numeric_prop(node: &Value, key: &str) -> Option<f64> {
    node.get(key).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
    })
}

#[path = "orchestration_self_check_card_image.rs"]
mod card_image;
pub(crate) use card_image::check_generated_nodes_for_prompt;

#[cfg(test)]
#[path = "orchestration_self_check_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "orchestration_self_check_scroller_tests.rs"]
mod tests_scroller;
