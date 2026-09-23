//! covers-backfill parse / repair passthrough tests — sibling module of
//! `plan.rs` to keep the spine under the 800-line file budget.

use super::*;
use crate::plan_repair::parse_orchestrator_response;

fn req(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        ..Default::default()
    }
}

/// Strict parse reads each subtask's `covers` verbatim — the coverage gate
/// depends on the exact brief wording surviving the parse unchanged.
#[test]
fn parse_plan_reads_covers_backfill() {
    let text = r##"```json
{
  "rootFrame": { "id": "page", "name": "Page", "width": 1200, "height": 0,
                 "layout": "vertical", "gap": 0 },
  "subtasks": [
    { "id": "hero", "label": "Hero Section", "covers": ["英雄"],
      "elements": "headline, CTA", "region": { "width": 1200, "height": 560 } },
    { "id": "pricing", "label": "Pricing Tiers", "covers": ["定价三档", "页脚"],
      "elements": "3 pricing cards", "region": { "width": 1200, "height": 360 } }
  ]
}
```"##;
    let plan = parse_plan(text).expect("parse");
    assert_eq!(
        plan.subtasks[0].covers,
        Some(vec!["英雄".to_string()]),
        "covers must survive the strict parse verbatim"
    );
    assert_eq!(
        plan.subtasks[1].covers,
        Some(vec!["定价三档".to_string(), "页脚".to_string()])
    );
}

/// A plan without `covers` (old models / old prompt) parses to `None`, never
/// to an empty vec — `None` is what keeps the gate on its legacy path.
#[test]
fn parse_plan_without_covers_leaves_none() {
    let text = r#"{ "rootFrame": { "id": "r", "name": "P", "width": 1200, "height": 0 },
                    "subtasks": [ { "id": "hero", "label": "Hero",
                                    "region": { "width": 1200, "height": 400 } } ] }"#;
    let plan = parse_plan(text).expect("parse");
    assert_eq!(plan.subtasks[0].covers, None);
    // An empty array degrades to None as well, not to an empty claim.
    let text = r#"{ "rootFrame": { "id": "r", "name": "P", "width": 1200, "height": 0 },
                    "subtasks": [ { "id": "hero", "label": "Hero", "covers": [],
                                    "region": { "width": 1200, "height": 400 } } ] }"#;
    let plan = parse_plan(text).expect("parse");
    assert_eq!(plan.subtasks[0].covers, None);
}

/// The repair path (schema-invalid JSON recovered by `coerce_subtask`) must
/// not drop the backfill — a repaired plan is the one that runs, and losing
/// its covers would re-arm the false-positive retry this field exists to kill.
#[test]
fn repair_path_preserves_covers() {
    // No rootFrame, no id, no region: strict parse fails, repair recovers.
    let raw = r#"{ "subtasks": [
        { "label": "Hero Section", "covers": ["英雄"] },
        { "name": "Pricing Tiers", "covers": ["定价三档"] }
    ] }"#;
    let (plan, repaired) =
        parse_orchestrator_response(raw, &req("产品官网：包含定价三档、页脚")).expect("repair");
    assert!(repaired, "fixture must exercise the repair path");
    assert_eq!(
        plan.subtasks[0].covers,
        Some(vec!["英雄".to_string()]),
        "repair must carry covers through verbatim"
    );
    assert_eq!(plan.subtasks[1].covers, Some(vec!["定价三档".to_string()]));
}

/// Serialization omits `covers` when `None`, so round-tripping a legacy plan
/// through JSON stays byte-compatible with the pre-field format.
#[test]
fn serialization_omits_absent_covers() {
    let text = r#"{ "rootFrame": { "id": "r", "name": "P", "width": 1200, "height": 0 },
                    "subtasks": [ { "id": "hero", "label": "Hero", "covers": ["英雄"],
                                    "region": { "width": 1200, "height": 400 } } ] }"#;
    let plan = parse_plan(text).expect("parse");
    let json = serde_json::to_string(&plan.subtasks[0]).expect("serialize");
    assert!(json.contains(r#""covers":["英雄"]"#), "got {json}");

    let mut legacy = plan;
    legacy.subtasks[0].covers = None;
    let json = serde_json::to_string(&legacy.subtasks[0]).expect("serialize");
    assert!(!json.contains("covers"), "got {json}");
}
