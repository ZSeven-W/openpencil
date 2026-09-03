//! `run.rs` inline tests — sequential + single-mode planning + 3-attempt ladder.
//!
//! Wired as `#[path = "run_tests.rs"] mod tests;` inside `run.rs`;
//! stays a child module of `run`, so `use super::*` resolves to `run`.

use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};

fn stub_providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    }
}

fn req() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

// Standard tier model id (used by the 3-attempt subtask ladder tests).
fn req_standard() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        // "gpt-4o" matches Standard tier in model_profile table
        model: Some("gpt-4o".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

// Basic tier model id — the ONLY tier where attempt 2's `reduced_complexity`
// flag has any effect at all (`compact_skills::apply_skill_filter`'s doc:
// "Basic tier only"), so it's the tier that can actually distinguish this
// module's quality-vs-transport retry-ladder split.
fn req_basic() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        // "glm-4-plus" matches Basic tier in model_profile table
        model: Some("glm-4-plus".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

const PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } },
    { "id": "feat", "label": "Features", "region": { "width": 1200, "height": 400 } }
  ]
}"##;

const MOBILE_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Mobile Page", "width": 390, "height": 844,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFF8F0" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 390, "height": 300 } }
  ]
}"##;

// Script-gen is the default subagent generation protocol, so the fixture is a
// JS program calling the bound `I(parent, obj)` recorder (a single insert
// whose object nests its children inline) rather than raw `_parent` JSONL.
// The batch_design executor reassigns fresh ids to every inserted node
// regardless of what's authored here, so callers must not assert on the
// literal "{prefix}-1" / "{prefix}-title" strings — the "content" field
// (which survives verbatim) is what identifies which section landed.
fn node_json(prefix: &str) -> String {
    format!(
        r#"I(null, {{"type":"frame","name":"Sec","x":0,"y":0,"width":1200,"height":300,"children":[{{"type":"text","content":"{prefix}","fontSize":18}}]}});"#
    )
}

// A radial ring whose progress arc (60px) is far smaller than its track
// (120px) — `orchestration_self_check`'s `radial-stack-not-concentric`
// flags this, and neither repair tier can auto-fix it (the arc-diameter
// mismatch is too implausible to guess a fix for; see
// `radial_preinsert_tests::explicit_but_unrepairable_radial_shapes_are_rejected_without_guessing`).
// This parses fine as script-gen — the rejection comes from self-check, not
// from a parse/stream failure — so it's the fixture for proving the retry
// ladder treats a QUALITY rejection differently from a transport failure.
fn radial_reject_script() -> String {
    r##"I(null, {"type":"frame","name":"Ring Section","x":0,"y":0,"width":1200,"height":300,"children":[
        {"type":"frame","name":"Steps Ring","width":120,"height":120,"children":[
            {"type":"ellipse","name":"track","width":120,"height":120,"innerRadius":0.82,"sweepAngle":360,"fill":[{"type":"solid","color":"#22C55E"}]},
            {"type":"ellipse","name":"progress","width":60,"height":60,"innerRadius":0.82,"startAngle":-90,"sweepAngle":264,"fill":[{"type":"solid","color":"#22C55E"}]},
            {"type":"frame","name":"centre","width":80,"height":44,"children":[{"type":"text","content":"64%"}]}
        ]}
    ]});"##
        .into()
}

fn existing_root_json(
    id: &str,
    name: &str,
    x: f64,
    y: f64,
    width: f64,
) -> jian_ops_schema::node::PenNode {
    serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": id,
        "name": name,
        "x": x,
        "y": y,
        "width": width,
        "height": 844.0,
        "layout": "vertical",
        "children": [
            {"type": "frame", "id": format!("{id}-content"), "name": "Content", "children": []}
        ]
    }))
    .expect("valid existing root")
}

// Cluster test modules — this file keeps the shared fixtures (plan JSON,
// scripted LLM helpers); each child mounts with `use super::*`.
#[path = "run_tests_core.rs"]
mod core_tests;
#[path = "run_tests_dashboard.rs"]
mod dashboard_tests;
#[path = "run_tests_geometry_salvage.rs"]
mod geometry_salvage_tests;
#[path = "run_tests_planning_retry.rs"]
mod planning_retry_tests;
