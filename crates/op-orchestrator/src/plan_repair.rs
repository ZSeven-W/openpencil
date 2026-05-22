//! `plan_repair` — JSON-repair value coercers.
//!
//! Task B1: helper functions ported from
//! `apps/web/src/services/ai/orchestrator-planning.ts:272-352`.
//! Task B2 (repair_plan_object + finalize_plan) and B3
//! (parse_orchestrator_response) will be appended in later tasks.

#![allow(dead_code)]

use crate::plan::PlanFill;
use serde_json::Value;

// ── public(crate) helpers ─────────────────────────────────────────────────────

/// Non-empty trimmed string, or `None`.
///
/// Port of TS `asString`.
pub(crate) fn as_string(value: &Value) -> Option<String> {
    let s = value.as_str()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Finite number strictly > 0, or `None`.
///
/// Port of TS `asPositiveNumber`.
pub(crate) fn as_positive_number(value: &Value) -> Option<f64> {
    let n = value.as_f64()?;
    if n.is_finite() && n > 0.0 {
        Some(n)
    } else {
        None
    }
}

/// Finite number ≥ 0, or `None`.
///
/// Port of TS `asNonNegativeNumber`.
pub(crate) fn as_non_negative_number(value: &Value) -> Option<f64> {
    let n = value.as_f64()?;
    if n.is_finite() && n >= 0.0 {
        Some(n)
    } else {
        None
    }
}

/// Validates that the value is one of `"none"`, `"vertical"`, or
/// `"horizontal"`, matching the layout field type on `RootFrameSpec`.
///
/// Port of TS `asLayout`.
pub(crate) fn as_layout(value: &Value) -> Option<String> {
    match value.as_str()? {
        "none" | "vertical" | "horizontal" => Some(value.as_str().unwrap().to_owned()),
        _ => None,
    }
}

/// Coerces a fill value:
/// - an array of `{type, color}` objects → `Vec<PlanFill>` (entries without
///   a valid `color` are dropped; `type` defaults to `"solid"`).
/// - a bare color string → single-entry `Vec<PlanFill>` with `type="solid"`.
/// - anything else → `None`.
///
/// Port of TS `coerceFill`.
pub(crate) fn coerce_fill(value: &Value) -> Option<Vec<PlanFill>> {
    if let Some(arr) = value.as_array() {
        let solids: Vec<PlanFill> = arr
            .iter()
            .filter(|entry| is_record(entry))
            .filter_map(|entry| {
                let color = as_string(&entry["color"])?;
                let kind = as_string(&entry["type"]).unwrap_or_else(|| "solid".to_owned());
                Some(PlanFill { kind, color })
            })
            .collect();
        if solids.is_empty() {
            None
        } else {
            Some(solids)
        }
    } else {
        let color = as_string(value)?;
        Some(vec![PlanFill {
            kind: "solid".to_owned(),
            color,
        }])
    }
}

/// Returns `true` when `value` is a JSON object (not null, not an array).
///
/// Port of TS `isRecord`.
pub(crate) fn is_record(value: &Value) -> bool {
    value.is_object()
}

/// Converts `label` to a safe section id:
/// - lowercase
/// - runs of `[^a-z0-9]` → `-`
/// - leading/trailing `-` stripped
/// - empty result → `"section-{index + 1}"` (1-based, matching TS)
///
/// Port of TS `makeSafeSectionId`.
pub(crate) fn make_safe_section_id(label: &str, index: usize) -> String {
    // Build lowercase, replacing any non-alphanumeric run with '-'
    let mut result = String::new();
    let lower = label.to_lowercase();
    let mut in_sep = false;
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            in_sep = false;
            result.push(ch);
        } else if !in_sep {
            in_sep = true;
            result.push('-');
        }
    }
    // Trim leading/trailing '-'
    let trimmed = result.trim_matches('-');
    if trimmed.is_empty() {
        format!("section-{}", index + 1)
    } else {
        trimmed.to_owned()
    }
}

/// Distributes `total_height` across `count` sections using a weighted
/// allocation and a remainder fix-up loop identical to the TS implementation.
///
/// Weights: first section 1.4×, last section (when count ≥ 3) 0.6×, others 1.0×.
/// Minimum section height: 80 px.
///
/// Port of TS `allocateSectionHeights`.
pub(crate) fn allocate_section_heights(total_height: i64, count: usize) -> Vec<i64> {
    if count == 0 {
        return vec![];
    }
    if count == 1 {
        return vec![total_height];
    }

    let min_height: i64 = 80;

    // Build weight array
    let weights: Vec<f64> = (0..count)
        .map(|i| {
            if i == 0 {
                1.4_f64
            } else if i == count - 1 && count >= 3 {
                0.6_f64
            } else {
                1.0_f64
            }
        })
        .collect();

    let total_weight: f64 = weights.iter().sum();

    let mut heights: Vec<i64> = weights
        .iter()
        .map(|&w| {
            let raw = ((total_height as f64) * w / total_weight).round() as i64;
            raw.max(min_height)
        })
        .collect();

    // Add-up fix-up: distribute surplus by round-robin from the middle
    let mut allocated: i64 = heights.iter().sum();
    let mut idx = count / 2; // floor(count / 2) — matches TS Math.floor
    while allocated < total_height {
        heights[idx] += 1;
        allocated += 1;
        idx = (idx + 1) % count;
    }

    // Subtract fix-up: trim from the end, respecting min_height
    let mut idx = count - 1;
    while allocated > total_height {
        if heights[idx] > min_height {
            heights[idx] -= 1;
            allocated -= 1;
        }
        if idx == 0 {
            idx = count - 1;
        } else {
            idx -= 1;
        }
    }

    heights
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    // ── as_string ─────────────────────────────────────────────────────────────

    #[test]
    fn as_string_non_empty_trimmed() {
        assert_eq!(as_string(&json!("  hello  ")), Some("hello".into()));
        assert_eq!(as_string(&json!("world")), Some("world".into()));
    }

    #[test]
    fn as_string_empty_or_whitespace_returns_none() {
        assert_eq!(as_string(&json!("")), None);
        assert_eq!(as_string(&json!("   ")), None);
    }

    #[test]
    fn as_string_non_string_returns_none() {
        assert_eq!(as_string(&json!(42)), None);
        assert_eq!(as_string(&Value::Null), None);
        assert_eq!(as_string(&json!(true)), None);
        assert_eq!(as_string(&json!([])), None);
    }

    // ── as_positive_number ────────────────────────────────────────────────────

    #[test]
    fn as_positive_number_accepts_finite_positive() {
        assert_eq!(as_positive_number(&json!(1.0)), Some(1.0));
        assert_eq!(as_positive_number(&json!(0.001)), Some(0.001));
        assert_eq!(as_positive_number(&json!(1200)), Some(1200.0));
    }

    #[test]
    fn as_positive_number_rejects_zero_and_negative() {
        assert_eq!(as_positive_number(&json!(0)), None);
        assert_eq!(as_positive_number(&json!(-5.0)), None);
    }

    #[test]
    fn as_positive_number_rejects_non_number() {
        assert_eq!(as_positive_number(&json!("1.5")), None);
        assert_eq!(as_positive_number(&Value::Null), None);
    }

    // ── as_non_negative_number ────────────────────────────────────────────────

    #[test]
    fn as_non_negative_number_accepts_zero_and_positive() {
        assert_eq!(as_non_negative_number(&json!(0)), Some(0.0));
        assert_eq!(as_non_negative_number(&json!(0.0)), Some(0.0));
        assert_eq!(as_non_negative_number(&json!(42.5)), Some(42.5));
    }

    #[test]
    fn as_non_negative_number_rejects_negative() {
        assert_eq!(as_non_negative_number(&json!(-0.001)), None);
        assert_eq!(as_non_negative_number(&json!(-100)), None);
    }

    #[test]
    fn as_non_negative_number_rejects_non_number() {
        assert_eq!(as_non_negative_number(&json!("0")), None);
        assert_eq!(as_non_negative_number(&Value::Null), None);
    }

    // ── as_layout ─────────────────────────────────────────────────────────────

    #[test]
    fn as_layout_accepts_valid_values() {
        assert_eq!(as_layout(&json!("none")), Some("none".into()));
        assert_eq!(as_layout(&json!("vertical")), Some("vertical".into()));
        assert_eq!(as_layout(&json!("horizontal")), Some("horizontal".into()));
    }

    #[test]
    fn as_layout_rejects_invalid_values() {
        assert_eq!(as_layout(&json!("flex")), None);
        assert_eq!(as_layout(&json!("")), None);
        assert_eq!(as_layout(&json!(42)), None);
        assert_eq!(as_layout(&Value::Null), None);
    }

    // ── coerce_fill ───────────────────────────────────────────────────────────

    #[test]
    fn coerce_fill_array_of_objects() {
        let v = json!([{"type": "solid", "color": "#fff"}]);
        let fills = coerce_fill(&v).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].kind, "solid");
        assert_eq!(fills[0].color, "#fff");
    }

    #[test]
    fn coerce_fill_array_defaults_type_to_solid() {
        let v = json!([{"color": "#123456"}]);
        let fills = coerce_fill(&v).unwrap();
        assert_eq!(fills[0].kind, "solid");
    }

    #[test]
    fn coerce_fill_bare_color_string() {
        let v = json!("#aabbcc");
        let fills = coerce_fill(&v).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].kind, "solid");
        assert_eq!(fills[0].color, "#aabbcc");
    }

    #[test]
    fn coerce_fill_array_drops_entries_without_color() {
        let v = json!([{"type": "solid"}, {"color": "#ff0000"}]);
        let fills = coerce_fill(&v).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].color, "#ff0000");
    }

    #[test]
    fn coerce_fill_empty_array_returns_none() {
        assert_eq!(coerce_fill(&json!([])), None);
    }

    #[test]
    fn coerce_fill_empty_string_returns_none() {
        assert_eq!(coerce_fill(&json!("")), None);
    }

    // ── is_record ─────────────────────────────────────────────────────────────

    #[test]
    fn is_record_plain_object() {
        assert!(is_record(&json!({"key": "val"})));
        assert!(is_record(&json!({})));
    }

    #[test]
    fn is_record_non_objects() {
        assert!(!is_record(&Value::Null));
        assert!(!is_record(&json!([])));
        assert!(!is_record(&json!("string")));
        assert!(!is_record(&json!(42)));
        assert!(!is_record(&json!(true)));
    }

    // ── make_safe_section_id ──────────────────────────────────────────────────

    #[test]
    fn make_safe_section_id_normal_label() {
        assert_eq!(make_safe_section_id("Hero Section", 0), "hero-section");
    }

    #[test]
    fn make_safe_section_id_multiple_spaces_and_special_chars() {
        assert_eq!(make_safe_section_id("FAQ  &  Help", 1), "faq-help");
    }

    #[test]
    fn make_safe_section_id_empty_after_strip() {
        assert_eq!(make_safe_section_id("!!!", 0), "section-1");
        assert_eq!(make_safe_section_id("", 2), "section-3");
    }

    #[test]
    fn make_safe_section_id_already_safe() {
        assert_eq!(make_safe_section_id("hero", 0), "hero");
    }

    #[test]
    fn make_safe_section_id_index_is_one_based_in_fallback() {
        // index=0 → "section-1", index=4 → "section-5"
        assert_eq!(make_safe_section_id("???", 0), "section-1");
        assert_eq!(make_safe_section_id("???", 4), "section-5");
    }

    // ── allocate_section_heights ──────────────────────────────────────────────

    #[test]
    fn allocate_section_heights_zero_count() {
        assert_eq!(allocate_section_heights(1080, 0), Vec::<i64>::new());
    }

    #[test]
    fn allocate_section_heights_single_section() {
        assert_eq!(allocate_section_heights(800, 1), vec![800]);
    }

    #[test]
    fn allocate_section_heights_two_sections_sums_correctly() {
        let total = 1080;
        let result = allocate_section_heights(total, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result.iter().sum::<i64>(), total);
        // Both sections should be ≥ min_height (80)
        assert!(result.iter().all(|&h| h >= 80));
    }

    #[test]
    fn allocate_section_heights_three_sections_sums_correctly() {
        let total = 1080;
        let result = allocate_section_heights(total, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result.iter().sum::<i64>(), total);
        assert!(result.iter().all(|&h| h >= 80));
        // First section should be larger than last (1.4 vs 0.6 weight)
        assert!(result[0] > result[2]);
    }

    #[test]
    fn allocate_section_heights_five_sections_sums_correctly() {
        let total = 2400;
        let result = allocate_section_heights(total, 5);
        assert_eq!(result.len(), 5);
        assert_eq!(result.iter().sum::<i64>(), total);
        assert!(result.iter().all(|&h| h >= 80));
    }

    #[test]
    fn allocate_section_heights_weighted_distribution() {
        // For 3 sections: weights are [1.4, 1.0, 0.6], total=3.0
        // With total=300: first≈140, middle≈100, last≈60 (min 80 applies to last)
        let result = allocate_section_heights(300, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result.iter().sum::<i64>(), 300);
        // First section gets the most weight
        assert!(result[0] > result[1]);
    }
}
