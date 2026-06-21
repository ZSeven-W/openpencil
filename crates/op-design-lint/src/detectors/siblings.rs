//! Sibling-consistency detectors ported from
//! `pen-ai-skills/src/diagnostics/detectors.ts`.
//!
//! Three detectors compare a property across `>= 3` same-kind siblings and
//! flag the values that disagree with the majority:
//! - `detect_sibling_inconsistencies` (+ `check_consistency`) — the
//!   strict/loose two-pass `height` / `cornerRadius` / `fontSize` check.
//! - `detect_mixed_sibling_corner_radius` — a stricter equal-or-report
//!   `cornerRadius` check (catches 8-vs-12 mismatches the `>= 50%`-delta
//!   sibling-inconsistency pass misses).
//! - `detect_mixed_sibling_padding` — the same for normalised `padding`.

use std::collections::HashSet;

use jian_ops_schema::node::{Padding, PenNode};
use serde_json::Value;

use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{
    children, corner_radius_numeric, fmt_num, json_number, node_id, node_kind_str,
    node_property_value, padding, raw_padding_value, role,
};

/// Properties whose consistency is role-dependent (a fixed mobile-chrome
/// `height` must NOT be compared against content-section heights). Checked
/// only in the strict, same-role pass.
const FRAME_STRICT_PROPS: &[&str] = &["height"];
/// Properties whose consistency is role-independent — compared in both the
/// strict and the loose pass.
const FRAME_LOOSE_PROPS: &[&str] = &["cornerRadius"];
/// The strict property set for `text` siblings.
const TEXT_STRICT_PROPS: &[&str] = &["fontSize"];

/// The `FixProperty` a sibling-inconsistency property name targets. The three
/// names in the `*_PROPS` consts are the only inputs.
fn fix_property_for(property: &str) -> FixProperty {
    match property {
        "cornerRadius" => FixProperty::CornerRadius,
        "fontSize" => FixProperty::FontSize,
        // `height` is the only remaining `FRAME_STRICT_PROPS` member.
        _ => FixProperty::Height,
    }
}

/// Port of `detectSiblingInconsistencies` (`detectors.ts:238-312`). Detects
/// property inconsistencies among siblings (`>= 3` siblings, `>= 2/3`
/// majority rule); each outlier gets an `Issue` to align with the majority.
///
/// Two parallel passes:
///   1. STRICT — groups by `type:role` and checks STRICT + LOOSE props.
///      Severity `warning`. Divider / spacer roles are skipped (intentionally
///      tiny layout primitives whose dimensions shouldn't match anything).
///   2. LOOSE — groups by `type` only and checks LOOSE props. Catches
///      singleton-role outliers (web landing-page sections). Severity `info`.
///
/// Issues overlapping between passes are deduped on `(node_id, property)` —
/// the strict-pass version wins because it is collected first.
pub fn detect_sibling_inconsistencies(root: &PenNode) -> Vec<Issue> {
    let mut raw = Vec::new();
    walk_sibling_inconsistencies(root, &mut raw);

    // Dedupe: strict and loose passes can both emit on the same
    // (node_id, property) pair (a same-role cornerRadius outlier is caught by
    // both). Keep the first occurrence — strict iterates first. The key is
    // `{node_id}:{property}` to match the TS dedup string exactly.
    let mut seen = HashSet::new();
    raw.into_iter()
        .filter(|issue| seen.insert(format!("{}:{}", issue.node_id, issue.property.wire_str())))
        .collect()
}

fn walk_sibling_inconsistencies(node: &PenNode, raw: &mut Vec<Issue>) {
    let kids = children(node);
    if kids.len() >= 3 {
        // Insertion-ordered group lists so the strict / loose passes iterate
        // groups in TS `Map` order.
        let mut strict_groups: Vec<(String, Vec<&PenNode>)> = Vec::new();
        let mut type_groups: Vec<(String, Vec<&PenNode>)> = Vec::new();
        for child in kids {
            let child_role = role(child).unwrap_or("").to_lowercase();
            // Skip divider / spacer — visual primitives whose dimensions are
            // intentionally tiny; reporting them as outliers is always noise.
            if child_role == "divider" || child_role == "spacer" {
                continue;
            }
            let role_key = if child_role.is_empty() {
                "__none__"
            } else {
                child_role.as_str()
            };
            let kind = node_kind_str(child);
            push_group(&mut strict_groups, format!("{kind}:{role_key}"), child);
            push_group(&mut type_groups, kind.to_string(), child);
        }

        // Strict pass: same-type, same-role siblings, every applicable
        // property (STRICT + LOOSE). Severity `warning`.
        for (strict_key, siblings) in &strict_groups {
            if siblings.len() < 3 {
                continue;
            }
            let kind = strict_key.split(':').next().unwrap_or("");
            let props: Vec<&str> = if kind == "text" {
                TEXT_STRICT_PROPS.to_vec()
            } else {
                FRAME_STRICT_PROPS
                    .iter()
                    .chain(FRAME_LOOSE_PROPS)
                    .copied()
                    .collect()
            };
            for prop in props {
                check_consistency(siblings, prop, IssueSeverity::Warning, raw);
            }
        }

        // Loose pass: same-type-only siblings, role-independent props only.
        // Catches singleton-role outliers. Severity `info`.
        for (kind, siblings) in &type_groups {
            if siblings.len() < 3 {
                continue;
            }
            if kind == "text" {
                continue; // text has no role-independent props
            }
            for prop in FRAME_LOOSE_PROPS {
                check_consistency(siblings, prop, IssueSeverity::Info, raw);
            }
        }
    }
    for child in kids {
        walk_sibling_inconsistencies(child, raw);
    }
}

/// Append `child` to the insertion-ordered group keyed by `key`.
fn push_group<'a>(groups: &mut Vec<(String, Vec<&'a PenNode>)>, key: String, child: &'a PenNode) {
    if let Some((_, bucket)) = groups.iter_mut().find(|(k, _)| *k == key) {
        bucket.push(child);
    } else {
        groups.push((key, vec![child]));
    }
}

/// Port of `checkConsistency` (`detectors.ts:726-765`). Groups `siblings` by
/// the canonical JSON of `property` (skipping nodes where the property is
/// absent / null), requires `>= 2` distinct values, requires the majority to
/// hold `>= 2/3` of the nodes-with-the-property, and flags every non-majority
/// node with a `sibling-inconsistency` issue.
fn check_consistency(
    siblings: &[&PenNode],
    property: &str,
    severity: IssueSeverity,
    issues: &mut Vec<Issue>,
) {
    // Insertion-ordered: the majority tie-break is strict `>` so the
    // first-inserted value wins a tie, matching TS `Map` iteration.
    struct Group {
        value: Value,
        nodes: Vec<String>,
    }
    let mut values: Vec<(String, Group)> = Vec::new();
    for node in siblings {
        let Some(raw) = node_property_value(node, property) else {
            continue;
        };
        // TS `raw == null` skip — `node_property_value` already returns `None`
        // for an absent field; a present-but-`Null` value is still skipped.
        if raw.is_null() {
            continue;
        }
        let key = canonical_json(&raw);
        if let Some((_, group)) = values.iter_mut().find(|(k, _)| *k == key) {
            group.nodes.push(node_id(node).to_string());
        } else {
            values.push((
                key,
                Group {
                    value: raw,
                    nodes: vec![node_id(node).to_string()],
                },
            ));
        }
    }
    if values.len() < 2 {
        return;
    }

    // Majority — first group with the most nodes (strict `>` keeps the first).
    let majority_idx = values.iter().enumerate().fold(0usize, |best, (i, (_, g))| {
        if g.nodes.len() > values[best].1.nodes.len() {
            i
        } else {
            best
        }
    });
    let total_with_prop: usize = values.iter().map(|(_, g)| g.nodes.len()).sum();
    let majority_count = values[majority_idx].1.nodes.len();
    // TS: `majority.nodes.length < (totalWithProp * 2) / 3` — float division.
    if (majority_count as f64) < (total_with_prop as f64) * 2.0 / 3.0 {
        return;
    }

    let majority_value = values[majority_idx].1.value.clone();
    let property_fix = fix_property_for(property);
    // A PILL cornerRadius (>= 40) is an INTENTIONAL design choice — a
    // fully-rounded active nav tab / toggle highlight — NOT a sibling
    // inconsistency. Skip cornerRadius normalization when the outlier OR the
    // majority is a pill, so this lint never flattens a rounded active nav tab
    // back to its square siblings (the active-nav-tab-reset-to-square bug) nor
    // rounds square siblings up to a pill. Card-scale radii (8–16) still
    // normalize. (Mirrors the guard in `detect_mixed_sibling_corner_radius`.)
    const PILL_RADIUS: f64 = 40.0;
    let is_corner = property == "cornerRadius";
    let majority_is_pill = is_corner
        && majority_value
            .as_f64()
            .map(|v| v >= PILL_RADIUS)
            .unwrap_or(false);
    for (idx, (_, group)) in values.iter().enumerate() {
        if idx == majority_idx {
            continue;
        }
        let group_is_pill = is_corner
            && group
                .value
                .as_f64()
                .map(|v| v >= PILL_RADIUS)
                .unwrap_or(false);
        if is_corner && (group_is_pill || majority_is_pill) {
            continue;
        }
        for node_id_str in &group.nodes {
            issues.push(Issue {
                node_id: node_id_str.clone(),
                category: IssueCategory::SiblingInconsistency,
                severity,
                property: property_fix,
                current_value: group.value.clone(),
                suggested_value: majority_value.clone(),
                reason: format!("inconsistent with {majority_count} siblings"),
            });
        }
    }
}

/// Canonical JSON string of a value — the Rust analogue of `JSON.stringify`
/// used as the sibling-grouping key. `serde_json::to_string` emits keys in a
/// stable order so structurally-equal values share a key.
fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Port of `detectMixedSiblingCornerRadius` (`detectors.ts:401-456`). Groups
/// children `>= 3` by `type:role` (skipping divider / spacer); within a group
/// counts `cornerRadius` values (absent → `0`), finds the modal, and — when
/// the modal holds `>= 60%` of the group — flags every sibling whose value
/// differs from the modal.
pub fn detect_mixed_sibling_corner_radius(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_mixed_corner_radius(root, &mut issues);
    issues
}

fn walk_mixed_corner_radius(node: &PenNode, issues: &mut Vec<Issue>) {
    let kids = children(node);
    if kids.len() >= 3 {
        for (_, siblings) in role_kind_groups(kids) {
            if siblings.len() < 3 {
                continue;
            }
            // Count cornerRadius values; absent → 0 (TS loose coercion).
            let mut counts: Vec<(f64, usize)> = Vec::new();
            for sibling in &siblings {
                let num = corner_radius_numeric(sibling);
                bump_count(&mut counts, num);
            }
            if counts.len() < 2 {
                continue;
            }
            // Modal — strict `>` keeps the first-inserted on a tie.
            let (modal, modal_count) =
                counts
                    .iter()
                    .copied()
                    .fold(
                        (0.0_f64, 0usize),
                        |(mv, mc), (v, c)| {
                            if c > mc {
                                (v, c)
                            } else {
                                (mv, mc)
                            }
                        },
                    );
            // Only flag a clear majority (`>= 60%`); a 1-1-1 split has no
            // canonical value to suggest.
            if (modal_count as f64) / (siblings.len() as f64) < 0.6 {
                continue;
            }
            for sibling in &siblings {
                let num = corner_radius_numeric(sibling);
                if num != modal {
                    // A PILL radius (>= 40) is an INTENTIONAL design choice — a
                    // fully-rounded active nav tab / toggle highlight — not a
                    // sibling inconsistency to flatten. Skip when either the
                    // sibling OR the modal is a pill, so the validator never
                    // resets a rounded active nav tab back to a square (the
                    // round-then-lint-flattens bug) nor forces square siblings
                    // into pills. Card-scale radii (8–16) still normalize.
                    const PILL_RADIUS: f64 = 40.0;
                    if num >= PILL_RADIUS || modal >= PILL_RADIUS {
                        continue;
                    }
                    issues.push(Issue {
                        node_id: node_id(sibling).to_string(),
                        category: IssueCategory::MixedSiblingCornerRadius,
                        severity: IssueSeverity::Warning,
                        property: FixProperty::CornerRadius,
                        current_value: json_number(num),
                        suggested_value: json_number(modal),
                        reason: format!(
                            "cornerRadius {} doesn't match the {} other {} sibling(s) at {}",
                            fmt_num(num),
                            modal_count,
                            kids.len() - 1,
                            fmt_num(modal),
                        ),
                    });
                }
            }
        }
    }
    for child in kids {
        walk_mixed_corner_radius(child, issues);
    }
}

/// Port of `detectMixedSiblingPadding` (`detectors.ts:552-625`). Like
/// [`detect_mixed_sibling_corner_radius`] but for `padding`, with a
/// `normalise` step that maps a number / 2-tuple / 4-tuple onto a canonical
/// `[t,r,b,l]` 4-tuple so shorthand and explicit padding compare equal.
pub fn detect_mixed_sibling_padding(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_mixed_padding(root, &mut issues);
    issues
}

fn walk_mixed_padding(node: &PenNode, issues: &mut Vec<Issue>) {
    let kids = children(node);
    if kids.len() >= 3 {
        for (_, siblings) in role_kind_groups(kids) {
            if siblings.len() < 3 {
                continue;
            }
            // Normalised padding string per sibling; only normalisable
            // paddings are counted.
            let norms: Vec<Option<[f64; 4]>> =
                siblings.iter().map(|s| normalise_padding(s)).collect();
            let mut counts: Vec<(String, usize)> = Vec::new();
            for norm in norms.iter().flatten() {
                bump_str_count(&mut counts, padding_key(norm));
            }
            if counts.len() < 2 {
                continue;
            }
            let (modal, modal_count) =
                counts
                    .iter()
                    .fold((String::new(), 0usize), |(mv, mc), (v, c)| {
                        if *c > mc {
                            (v.clone(), *c)
                        } else {
                            (mv, mc)
                        }
                    });
            // Same 60% rule as cornerRadius.
            if (modal_count as f64) / (siblings.len() as f64) < 0.6 {
                continue;
            }
            // Parse the modal `[a,b,c,d]` key back to a value to suggest.
            let Some(modal_tuple) = parse_padding_key(&modal) else {
                continue;
            };
            let suggested = padding_suggested_value(&modal_tuple);
            for (sibling, norm) in siblings.iter().zip(&norms) {
                let Some(norm_tuple) = norm else { continue };
                let norm_key = padding_key(norm_tuple);
                if norm_key != modal {
                    issues.push(Issue {
                        node_id: node_id(sibling).to_string(),
                        category: IssueCategory::MixedSiblingPadding,
                        severity: IssueSeverity::Warning,
                        property: FixProperty::Padding,
                        current_value: raw_padding_value(padding(sibling)),
                        suggested_value: suggested.clone(),
                        reason: format!(
                            "padding {norm_key} doesn't match the {modal_count} other sibling(s) at {modal}"
                        ),
                    });
                }
            }
        }
    }
    for child in kids {
        walk_mixed_padding(child, issues);
    }
}

/// Group children by `type:role`, skipping divider / spacer roles, in
/// insertion order. Shared by the two mixed-sibling detectors.
fn role_kind_groups(kids: &[PenNode]) -> Vec<(String, Vec<&PenNode>)> {
    let mut groups: Vec<(String, Vec<&PenNode>)> = Vec::new();
    for child in kids {
        let child_role = role(child).unwrap_or("").to_lowercase();
        if child_role == "divider" || child_role == "spacer" {
            continue;
        }
        let role_key = if child_role.is_empty() {
            "__none__"
        } else {
            child_role.as_str()
        };
        let key = format!("{}:{}", node_kind_str(child), role_key);
        push_group(&mut groups, key, child);
    }
    groups
}

/// Increment the `value`'s count in an insertion-ordered numeric count list.
fn bump_count(counts: &mut Vec<(f64, usize)>, value: f64) {
    if let Some((_, c)) = counts.iter_mut().find(|(v, _)| *v == value) {
        *c += 1;
    } else {
        counts.push((value, 1));
    }
}

/// Increment the `key`'s count in an insertion-ordered string count list.
fn bump_str_count(counts: &mut Vec<(String, usize)>, key: String) {
    if let Some((_, c)) = counts.iter_mut().find(|(k, _)| *k == key) {
        *c += 1;
    } else {
        counts.push((key, 1));
    }
}

/// Port of the TS `normalise` closure. A jian `Padding`:
/// - `Uniform(v)` → `[v,v,v,v]`
/// - `XY([v,h])`  → `[v,h,v,h]`
/// - `LtrB(t,r,b,l)` → itself
/// - `Expression(_)` → `None` ("anything else → no padding")
///
/// Audit footnote 8: jian `Padding` has NO 1-element variant, so the TS
/// `p.length === 1` branch has no jian equivalent and is intentionally absent.
fn normalise_padding(node: &PenNode) -> Option<[f64; 4]> {
    match padding(node)? {
        Padding::Uniform(v) => Some([*v, *v, *v, *v]),
        Padding::XY([v, h]) => Some([*v, *h, *v, *h]),
        Padding::LtrB(t) => Some(*t),
        Padding::Expression(_) => None,
    }
}

/// Canonical `[a,b,c,d]` string of a normalised padding tuple — the grouping
/// key, matching the TS `` `[${p[0]},${p[1]},${p[2]},${p[3]}]` `` form.
fn padding_key(tuple: &[f64; 4]) -> String {
    format!(
        "[{},{},{},{}]",
        fmt_num(tuple[0]),
        fmt_num(tuple[1]),
        fmt_num(tuple[2]),
        fmt_num(tuple[3]),
    )
}

/// Parse a `[a,b,c,d]` padding key back into a 4-tuple.
fn parse_padding_key(key: &str) -> Option<[f64; 4]> {
    let inner = key.strip_prefix('[')?.strip_suffix(']')?;
    let parts: Vec<f64> = inner.split(',').filter_map(|p| p.parse().ok()).collect();
    if parts.len() == 4 {
        Some([parts[0], parts[1], parts[2], parts[3]])
    } else {
        None
    }
}

/// The `suggested_value` for a modal padding: a single number when all four
/// sides are equal, else a 4-element array — matching the TS `allEqual`
/// branch.
fn padding_suggested_value(tuple: &[f64; 4]) -> Value {
    if tuple.iter().all(|v| *v == tuple[0]) {
        json_number(tuple[0])
    } else {
        Value::Array(tuple.iter().map(|v| json_number(*v)).collect())
    }
}
