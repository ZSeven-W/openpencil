//! `plan_repair` tests — split from `plan_repair.rs` to keep that file
//! under the 800-line cap.
//!
//! Wired as `#[path = "plan_repair_tests.rs"] mod tests;` inside
//! `plan_repair.rs`, so this stays the `plan_repair::tests` module and
//! `use super::*` resolves to `plan_repair`.

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

// ── Task B2: repair_plan_object ───────────────────────────────────────────

fn req(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        model: None,
        provider: None,
        design_md: None,
    }
}

fn req_with_design_md(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        model: None,
        provider: None,
        design_md: Some(jian_ops_schema::DesignMdSpec {
            raw: String::new(),
            project_name: None,
            visual_theme: Some("light".into()),
            color_palette: Some(vec![jian_ops_schema::DesignMdColor {
                name: "Background".into(),
                hex: "#F0F4F8".into(),
                role: "page background".into(),
            }]),
            typography: None,
            component_styles: None,
            layout_principles: None,
            generation_notes: None,
        }),
    }
}

/// Helper: `sections` key instead of `subtasks` should be accepted.
#[test]
fn repair_plan_object_sections_alias_works() {
    let obj = json!({
        "rootFrame": { "id": "root", "name": "Page", "width": 1200.0, "height": 800.0 },
        "sections": [
            { "id": "hero", "label": "Hero", "region": { "width": 1200.0, "height": 400.0 } }
        ]
    });
    let plan = repair_plan_object(&obj, &req("a landing page")).expect("repair should succeed");
    assert_eq!(plan.subtasks.len(), 1);
    assert_eq!(plan.subtasks[0].id, "hero");
}

/// When there is no `rootFrame` key, fields should be read off the top-level.
#[test]
fn repair_plan_object_missing_root_frame_reads_top_level() {
    let obj = json!({
        "id": "root",
        "name": "Page",
        "width": 1200.0,
        "height": 800.0,
        "subtasks": [
            { "id": "hero", "label": "Hero", "region": { "width": 1200.0, "height": 400.0 } }
        ]
    });
    let plan = repair_plan_object(&obj, &req("a landing page")).expect("repair should succeed");
    assert_eq!(plan.root_frame.id, "root");
    assert_eq!(plan.root_frame.width, 1200.0);
}

/// Bare string subtasks should become labelled subtasks.
#[test]
fn repair_plan_object_bare_string_subtasks_become_labelled() {
    let obj = json!({
        "rootFrame": { "id": "root", "name": "Page", "width": 390.0, "height": 812.0 },
        "subtasks": ["Hero", "Features", "Footer"]
    });
    let plan = repair_plan_object(&obj, &req("a mobile app")).expect("repair should succeed");
    assert_eq!(plan.subtasks.len(), 3);
    assert_eq!(plan.subtasks[0].label, "Hero");
    assert_eq!(plan.subtasks[0].id, "hero");
    assert_eq!(plan.subtasks[1].label, "Features");
}

/// `style_guide` (snake_case) should be treated as `styleGuideName` alias.
#[test]
fn repair_plan_object_style_guide_snake_case_alias() {
    let obj = json!({
        "rootFrame": { "id": "root", "name": "P", "width": 1200.0, "height": 800.0 },
        "subtasks": [{ "id": "s1", "label": "Section 1", "region": { "width": 1200.0, "height": 400.0 } }],
        "style_guide": "my-dark-theme"
    });
    let plan = repair_plan_object(&obj, &req("a page")).expect("repair should succeed");
    assert_eq!(plan.style_guide_name.as_deref(), Some("my-dark-theme"));
}

/// When all subtask objects are empty (no label, no valid fields), the
/// repair should still produce a plan using fallback labels.
#[test]
fn repair_plan_object_all_empty_objects_get_fallback_labels() {
    let obj = json!({
        "rootFrame": { "id": "root", "name": "P", "width": 1200.0, "height": 800.0 },
        "subtasks": [{}, {}]
    });
    // Empty objects get "Section N" labels via the fallback — not None.
    let plan = repair_plan_object(&obj, &req("a page"))
        .expect("repair should succeed with fallback labels");
    assert_eq!(plan.subtasks.len(), 2);
    assert_eq!(plan.subtasks[0].label, "Section 1");
    assert_eq!(plan.subtasks[1].label, "Section 2");
}

/// An object with zero subtask candidates should return `None`.
#[test]
fn repair_plan_object_no_subtasks_returns_none() {
    let obj = json!({
        "rootFrame": { "id": "root", "name": "P", "width": 1200.0, "height": 800.0 }
    });
    assert!(repair_plan_object(&obj, &req("a page")).is_none());
}

/// `elements`, `scope`, and `description` aliasing in subtask coercion.
#[test]
fn coerce_subtask_elements_scope_description_aliasing() {
    // `elements` present
    let candidate = json!({ "label": "Hero", "elements": "headline, CTA" });
    let st = coerce_subtask(&candidate, 0, 1200.0, 400).unwrap();
    assert_eq!(st.elements.as_deref(), Some("headline, CTA"));

    // `scope` fallback when `elements` absent
    let candidate2 = json!({ "label": "Hero", "scope": "nav bar" });
    let st2 = coerce_subtask(&candidate2, 0, 1200.0, 400).unwrap();
    assert_eq!(st2.elements.as_deref(), Some("nav bar"));

    // `description` fallback
    let candidate3 = json!({ "label": "Hero", "description": "A big banner" });
    let st3 = coerce_subtask(&candidate3, 0, 1200.0, 400).unwrap();
    assert_eq!(st3.elements.as_deref(), Some("A big banner"));
}

/// `screen` / `page` aliasing.
#[test]
fn coerce_subtask_screen_page_aliasing() {
    let c1 = json!({ "label": "X", "screen": "Home" });
    assert_eq!(
        coerce_subtask(&c1, 0, 1200.0, 400)
            .unwrap()
            .screen
            .as_deref(),
        Some("Home")
    );
    let c2 = json!({ "label": "X", "page": "Settings" });
    assert_eq!(
        coerce_subtask(&c2, 0, 1200.0, 400)
            .unwrap()
            .screen
            .as_deref(),
        Some("Settings")
    );
}

/// `elements` as array → joined with ", ".
#[test]
fn coerce_subtask_elements_array_joined() {
    let c = json!({ "label": "Hero", "elements": ["header", "nav", "CTA"] });
    let st = coerce_subtask(&c, 0, 1200.0, 400).unwrap();
    assert_eq!(st.elements.as_deref(), Some("header, nav, CTA"));
}

// ── Task B2: finalize_plan ────────────────────────────────────────────────

/// When `design_md` is present, `finalize_plan` must force
/// `style_guide_name = "design-md-custom"` and overwrite `root_frame.fill`
/// with the design.md background.
#[test]
fn finalize_plan_design_md_overrides_style_guide_and_fill() {
    let obj = json!({
        "rootFrame": { "id": "root", "name": "Page", "width": 1200.0, "height": 800.0 },
        "subtasks": [{ "id": "hero", "label": "Hero", "region": { "width": 1200.0, "height": 400.0 } }]
    });
    let plan = repair_plan_object(&obj, &req_with_design_md("a landing page"))
        .expect("repair should succeed");
    assert_eq!(plan.style_guide_name.as_deref(), Some("design-md-custom"));
    let fill = plan.root_frame.fill.as_ref().unwrap();
    assert_eq!(fill.len(), 1);
    // The design.md palette entry with role "page background" → #F0F4F8
    assert_eq!(fill[0].color, "#F0F4F8");
}

/// When `design_md` is absent and no `styleGuideName` is provided, the plan
/// is returned as-is (style_guide_name may be None or from fallback).
#[test]
fn finalize_plan_no_design_md_preserves_style_guide_name() {
    let obj = json!({
        "rootFrame": { "id": "root", "name": "P", "width": 1200.0, "height": 800.0 },
        "subtasks": [{ "id": "s", "label": "S", "region": { "width": 1200.0, "height": 400.0 } }],
        "styleGuideName": "clean-minimal-light"
    });
    let plan = repair_plan_object(&obj, &req("a page")).expect("repair should succeed");
    assert_eq!(
        plan.style_guide_name.as_deref(),
        Some("clean-minimal-light")
    );
}

// ── Task B2: build_fallback_heights ───────────────────────────────────────

#[test]
fn build_fallback_heights_empty() {
    let fallback = build_fallback_plan(&req("x"));
    assert!(build_fallback_heights(&fallback, 0).is_empty());
}

#[test]
fn build_fallback_heights_desktop_allocates_correctly() {
    // build_fallback_plan returns width=1200; height=360 for 1-section.
    // Desktop path → allocate_section_heights.
    let fallback = build_fallback_plan(&req("x"));
    let heights = build_fallback_heights(&fallback, 3);
    assert_eq!(heights.len(), 3);
    // Should sum to total height (section_count * SECTION_HEIGHT = 3 * 360)
    let total: i64 = heights.iter().sum();
    assert!(total > 0);
    assert!(heights.iter().all(|&h| h >= 80));
}

// ── Task B3: parse_orchestrator_response ─────────────────────────────────────

/// A minimal valid plan JSON suitable for building test inputs.
fn minimal_plan_json() -> &'static str {
    r#"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800 },
  "styleGuideName": "clean-light",
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } }
  ]
}"#
}

/// Clean JSON plan → `Some((plan, repaired=false))`.
#[test]
fn parse_orchestrator_response_clean_json_direct() {
    let raw = minimal_plan_json();
    let result = parse_orchestrator_response(raw, &req("a landing page"));
    let (plan, repaired) = result.expect("should parse");
    assert!(!repaired, "clean JSON should not be marked as repaired");
    assert_eq!(plan.root_frame.id, "root");
    assert_eq!(plan.subtasks.len(), 1);
    assert_eq!(plan.subtasks[0].id, "hero");
}

/// Same JSON wrapped in a ```json fence → `Some((_, repaired=false))`.
#[test]
fn parse_orchestrator_response_fenced_code_block() {
    let raw = format!("```json\n{}\n```", minimal_plan_json());
    let result = parse_orchestrator_response(&raw, &req("a landing page"));
    let (plan, repaired) = result.expect("should parse fenced JSON");
    assert!(!repaired, "valid fenced JSON should not be repaired");
    assert_eq!(plan.root_frame.id, "root");
    assert_eq!(plan.subtasks.len(), 1);
}

/// Plain ``` fence (no "json" tag) should also work.
#[test]
fn parse_orchestrator_response_fenced_no_lang_tag() {
    let raw = format!("```\n{}\n```", minimal_plan_json());
    let result = parse_orchestrator_response(&raw, &req("a landing page"));
    let (plan, repaired) = result.expect("should parse plain-fenced JSON");
    assert!(!repaired);
    assert_eq!(plan.root_frame.id, "root");
}

/// Prose before/after `{...}` → brace-slice strategy recovers it.
#[test]
fn parse_orchestrator_response_brace_slice_extracts_from_prose() {
    let raw = format!(
        "Here is the plan for your design:\n{}\nHope that helps!",
        minimal_plan_json()
    );
    let result = parse_orchestrator_response(&raw, &req("a landing page"));
    let (plan, repaired) = result.expect("brace-slice should recover JSON");
    assert!(!repaired, "brace-sliced valid JSON should not be repaired");
    assert_eq!(plan.root_frame.id, "root");
}

/// An alias-keyed plan (`sections` instead of `subtasks`) goes through the
/// repair path → `Some((_, repaired=true))`.
#[test]
fn parse_orchestrator_response_alias_key_repaired() {
    let raw = r#"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800 },
  "sections": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } }
  ]
}"#;
    let result = parse_orchestrator_response(raw, &req("a landing page"));
    let (plan, repaired) = result.expect("alias plan should be repaired");
    assert!(repaired, "sections-alias plan should be marked as repaired");
    assert_eq!(plan.subtasks.len(), 1);
}

/// Pure garbage → `None`.
#[test]
fn parse_orchestrator_response_garbage_returns_none() {
    assert!(
        parse_orchestrator_response("I cannot generate a plan.", &req("x")).is_none(),
        "non-JSON garbage must return None"
    );
    assert!(
        parse_orchestrator_response("", &req("x")).is_none(),
        "empty string must return None"
    );
}

/// A fenced alias-keyed plan gets repaired → `repaired=true`.
#[test]
fn parse_orchestrator_response_fenced_alias_repaired() {
    let inner = r#"{
  "rootFrame": { "id": "r", "name": "P", "width": 390, "height": 812 },
  "tasks": [
    { "id": "s1", "label": "Screen 1", "region": { "width": 390, "height": 400 } }
  ]
}"#;
    let raw = format!("```json\n{inner}\n```");
    let result = parse_orchestrator_response(&raw, &req("mobile app"));
    let (plan, repaired) = result.expect("fenced alias should be repaired");
    assert!(
        repaired,
        "alias-key plan in fence should be marked repaired"
    );
    assert_eq!(plan.subtasks.len(), 1);
}
