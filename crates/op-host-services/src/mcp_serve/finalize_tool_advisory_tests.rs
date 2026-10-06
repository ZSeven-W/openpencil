//! `finalize_design` advisory + informational-channel tests (structure
//! drift, board trailing void, board format drift, shader findings) —
//! split out of `finalize_tool_tests.rs` at the 800-line cap.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::EditorCommand;
use op_mcp::{McpTool, ToolOutcome};

use super::finalize_tool::finalize_design_snapshot;
use super::finalize_tool_tests::{find, load, load_fixture};

// ── DS P2-a item ③: structure-drift advisories ──────────────────────────────

/// A card board whose five "法则" items carry FIVE different internal
/// structures — the exact shape the pre-insertion self-check's
/// `sibling_structure_drift` detector flags. Nothing else in the fixture
/// needs repairing, so the advisory is the only structure signal.
const DRIFT_FIXTURE: &str = r##"{
  "version": "1.0",
  "children": [
    {
      "type": "frame",
      "id": "root",
      "name": "知识卡片",
      "width": 1080,
      "height": 1440,
      "layout": "vertical",
      "children": [
        { "type": "frame", "id": "i1", "name": "法则 01", "width": 900, "height": 100, "layout": "vertical",
          "children": [ { "type": "text", "id": "t1", "name": "Title", "content": "x" } ] },
        { "type": "frame", "id": "i2", "name": "法则 02", "width": 900, "height": 100, "layout": "vertical",
          "children": [ { "type": "rectangle", "id": "r2", "name": "Badge", "width": 40, "height": 40 } ] },
        { "type": "frame", "id": "i3", "name": "法则 03", "width": 900, "height": 100, "layout": "vertical",
          "children": [
            { "type": "text", "id": "t3a", "name": "Title", "content": "x" },
            { "type": "text", "id": "t3b", "name": "Body", "content": "x" }
          ] },
        { "type": "frame", "id": "i4", "name": "法则 04", "width": 900, "height": 100, "layout": "vertical",
          "children": [
            { "type": "rectangle", "id": "r4", "name": "Badge", "width": 40, "height": 40 },
            { "type": "text", "id": "t4", "name": "Title", "content": "x" }
          ] },
        { "type": "frame", "id": "i5", "name": "法则 05", "width": 900, "height": 100, "layout": "vertical",
          "children": [
            { "type": "text", "id": "t5", "name": "Title", "content": "x" },
            { "type": "rectangle", "id": "r5", "name": "Badge", "width": 40, "height": 40 }
          ] }
      ]
    }
  ]
}"##;

/// The same board with five ISOMORPHIC items — the negative fixture: no
/// family drifts, so the advisory list must come back empty.
const ISOMORPHIC_FIXTURE: &str = r##"{
  "version": "1.0",
  "children": [
    {
      "type": "frame",
      "id": "root",
      "name": "知识卡片",
      "width": 1080,
      "height": 1440,
      "layout": "vertical",
      "children": [
        { "type": "frame", "id": "i1", "name": "法则 01", "width": 900, "height": 264, "layout": "vertical",
          "children": [ { "type": "text", "id": "t1", "name": "Title", "content": "x" } ] },
        { "type": "frame", "id": "i2", "name": "法则 02", "width": 900, "height": 264, "layout": "vertical",
          "children": [ { "type": "text", "id": "t2", "name": "Title", "content": "x" } ] },
        { "type": "frame", "id": "i3", "name": "法则 03", "width": 900, "height": 264, "layout": "vertical",
          "children": [ { "type": "text", "id": "t3", "name": "Title", "content": "x" } ] },
        { "type": "frame", "id": "i4", "name": "法则 04", "width": 900, "height": 264, "layout": "vertical",
          "children": [ { "type": "text", "id": "t4", "name": "Title", "content": "x" } ] },
        { "type": "frame", "id": "i5", "name": "法则 05", "width": 900, "height": 264, "layout": "vertical",
          "children": [ { "type": "text", "id": "t5", "name": "Title", "content": "x" } ] }
      ]
    }
  ]
}"##;

/// DFS pre-order sequence of `"type"` values — the structural fingerprint
/// the drift detector compares. Geometry repairs may resize nodes, but they
/// never change this sequence.
fn type_sequence(node: &PenNode) -> String {
    let value = serde_json::to_value(node).expect("serialize node");
    let mut seq = String::new();
    push_types(&value, &mut seq);
    seq
}

fn push_types(value: &serde_json::Value, seq: &mut String) {
    if !seq.is_empty() {
        seq.push(' ');
    }
    seq.push_str(value.get("type").and_then(|t| t.as_str()).unwrap_or("?"));
    if let Some(children) = value.get("children").and_then(|c| c.as_array()) {
        for child in children {
            push_types(child, seq);
        }
    }
}

fn advisories_of(json: &str) -> Vec<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(json).expect("summary json");
    value["advisories"].as_array().cloned().unwrap_or_default()
}

fn complete_of(json: &str) -> bool {
    let value: serde_json::Value = serde_json::from_str(json).expect("summary json");
    value["complete"].as_bool().expect("complete is a boolean")
}

#[test]
fn finalize_reports_structure_drift_as_an_advisory_not_a_repair() {
    let mut live = load_fixture(DRIFT_FIXTURE);
    let before: Vec<String> = ["i1", "i2", "i3", "i4", "i5"]
        .iter()
        .map(|id| type_sequence(find(live.active_children(), id).expect("item present")))
        .collect();
    let tool = finalize_design_snapshot(&live);

    let outcome = tool.call(&BTreeMap::new());
    let (json, commands) = match outcome {
        ToolOutcome::OkJsonWithCommand(json, EditorCommand::Batch { commands }) => (json, commands),
        ToolOutcome::OkJson(json) => (json, Vec::new()),
        other => panic!("unexpected finalize outcome: {other:?}"),
    };

    // The drift advisory is scoped by code: the sparse card fixture also
    // legitimately carries a board-trailing-void advisory (DS P2-b item C),
    // which is a different finding.
    let advisories = advisories_of(&json);
    let drift: Vec<&serde_json::Value> = advisories
        .iter()
        .filter(|advisory| advisory["code"].as_str() == Some("section-structure-drift"))
        .collect();
    assert_eq!(drift.len(), 1, "exactly one drift advisory: {json}");
    let advisory = drift[0];
    let ids: Vec<&str> = advisory["nodeIds"]
        .as_array()
        .expect("nodeIds is an array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["i1", "i2", "i3", "i4", "i5"],
        "drift members: {json}"
    );
    assert!(
        advisory["message"]
            .as_str()
            .is_some_and(|m| m.contains("5") && m.contains("different")),
        "the advisory message names the family: {json}"
    );
    // The same fixture is a sparse card board (5x100 content on 1440), so
    // the P2-b void advisory rides the channel alongside the drift one.
    // (100px per section, not 120: the P2-c per-edge vertical floor leaves
    // the unproven bottom edge at 0, which gives the centred 5x120 block a
    // 24.7% trailing void — just under the 25% advisory floor; 5x100 keeps
    // the fixture honestly sparse above it.)
    let voids: Vec<&serde_json::Value> = advisories
        .iter()
        .filter(|advisory| advisory["code"].as_str() == Some("board-trailing-void"))
        .collect();
    assert_eq!(
        voids.len(),
        1,
        "the sparse card must also void-advise: {json}"
    );
    assert!(
        !complete_of(&json),
        "advisories must block completion: {json}"
    );
    assert_eq!(
        voids[0]["nodeIds"].as_array().map(|ids| ids.len()),
        Some(1),
        "the void advisory names the board root: {json}"
    );

    // The advisory is NOT part of the repair tally: repairs == itemized
    // repair records, and the advisory adds nothing to either.
    let value: serde_json::Value = serde_json::from_str(&json).expect("summary json");
    assert_eq!(
        value["repairs"].as_u64().unwrap_or(0) as usize,
        value["repairRecords"]
            .as_array()
            .map(|r| r.len())
            .unwrap_or(0),
        "repairs must count only repair records, never advisories: {json}"
    );

    // The advisory never rewrites the document: apply the returned batch and
    // the five items keep their (drifting) structures untouched.
    if !commands.is_empty() {
        assert!(
            live.apply(EditorCommand::Batch { commands }),
            "the recorded cleanup batch must apply cleanly"
        );
    }
    for (id, expected) in ["i1", "i2", "i3", "i4", "i5"].iter().zip(before) {
        assert_eq!(
            type_sequence(find(live.active_children(), id).expect("item present")),
            expected,
            "the advisory must not modify the document (item {id} changed)"
        );
    }
}

#[test]
fn finalize_reports_no_advisory_for_an_isomorphic_family() {
    let live = load_fixture(ISOMORPHIC_FIXTURE);
    let tool = finalize_design_snapshot(&live);
    let outcome = tool.call(&BTreeMap::new());
    let json = match outcome {
        ToolOutcome::OkJsonWithCommand(json, _) | ToolOutcome::OkJson(json) => json,
        other => panic!("unexpected finalize outcome: {other:?}"),
    };
    assert_eq!(
        advisories_of(&json).len(),
        0,
        "an isomorphic FULL board must produce an empty advisory list: {json}"
    );
    assert!(complete_of(&json), "a clean full board is complete: {json}");
}

// ── DS P2-b item C: board-trailing-void advisories ──────────────────────────

/// A card board whose only content is one 432px section on a 1440px board:
/// even after the cleanup centre repair halves the void, ~30% of the board
/// stays empty — content the repairs cannot add. The advisory must say so.
const VOID_FIXTURE: &str = r##"{
  "version": "1.0",
  "children": [
    {
      "type": "frame",
      "id": "root",
      "name": "知识卡片",
      "width": 1080,
      "height": 1440,
      "layout": "vertical",
      "children": [
        { "type": "frame", "id": "body", "name": "Body", "width": 900, "height": 432,
          "layout": "vertical",
          "children": [ { "type": "text", "id": "t1", "name": "Title", "content": "x" } ] }
      ]
    }
  ]
}"##;

/// The same board with content that reaches the bottom margin — the negative
/// fixture: no trailing void, so no void advisory may come back.
const FULL_FIXTURE: &str = r##"{
  "version": "1.0",
  "children": [
    {
      "type": "frame",
      "id": "root",
      "name": "知识卡片",
      "width": 1080,
      "height": 1440,
      "layout": "vertical",
      "children": [
        { "type": "frame", "id": "body", "name": "Body", "width": 900, "height": 1350,
          "layout": "vertical",
          "children": [ { "type": "text", "id": "t1", "name": "Title", "content": "x" } ] }
      ]
    }
  ]
}"##;

#[test]
fn finalize_reports_a_trailing_void_advisory_for_a_sparse_card_board() {
    let live = load_fixture(VOID_FIXTURE);
    let tool = finalize_design_snapshot(&live);
    let outcome = tool.call(&BTreeMap::new());
    let json = match outcome {
        ToolOutcome::OkJsonWithCommand(json, _) | ToolOutcome::OkJson(json) => json,
        other => panic!("unexpected finalize outcome: {other:?}"),
    };

    let value: serde_json::Value = serde_json::from_str(&json).expect("summary json");
    // The centre repair ran (its checkpoint shows in the itemized records)…
    assert!(
        value["repairRecords"]
            .as_array()
            .is_some_and(|records| records
                .iter()
                .any(|record| { record["pass"].as_str() == Some("card-board-centre") })),
        "the centre repair must run on the sparse card: {json}"
    );
    // …and the void that survives it is reported, not repaired.
    let advisories = advisories_of(&json);
    let voids: Vec<&serde_json::Value> = advisories
        .iter()
        .filter(|advisory| advisory["code"].as_str() == Some("board-trailing-void"))
        .collect();
    assert_eq!(voids.len(), 1, "exactly one void advisory: {json}");
    let advisory = voids[0];
    assert_eq!(
        advisory["nodeIds"]
            .as_array()
            .and_then(|ids| ids.first())
            .and_then(|v| v.as_str()),
        Some("root"),
        "the advisory names the board root: {json}"
    );
    let message = advisory["message"].as_str().expect("message");
    assert!(
        message.contains('%') && message.contains("add content or scale up type/spacing"),
        "the message names the void percentage and the fix direction: {json}"
    );
    assert!(
        !complete_of(&json),
        "trailing void must block completion: {json}"
    );
}

#[test]
fn finalize_reports_no_void_advisory_for_a_full_card_board() {
    let live = load_fixture(FULL_FIXTURE);
    let tool = finalize_design_snapshot(&live);
    let outcome = tool.call(&BTreeMap::new());
    let json = match outcome {
        ToolOutcome::OkJsonWithCommand(json, _) | ToolOutcome::OkJson(json) => json,
        other => panic!("unexpected finalize outcome: {other:?}"),
    };
    let advisories = advisories_of(&json);
    let voids: Vec<&serde_json::Value> = advisories
        .iter()
        .filter(|advisory| advisory["code"].as_str() == Some("board-trailing-void"))
        .collect();
    assert!(
        voids.is_empty(),
        "a full board must produce no void advisory: {json}"
    );
    assert!(complete_of(&json), "a full board is complete: {json}");
}

// ── DS P2-d item ②: card format-drift advisories ────────────────────────────

/// A card board grown from 3:4 to 1080x2116 by the text-wrap reflow — the
/// measured 0815 regen shape. The format-drift advisory must ride the same
/// echo-only channel, informational only.
const FORMAT_DRIFT_FIXTURE: &str = r##"{
  "version": "1.0",
  "children": [
    {
      "type": "frame",
      "id": "root",
      "name": "知识卡片",
      "width": 1080,
      "height": 2116,
      "layout": "vertical",
      "children": [
        { "type": "frame", "id": "body", "name": "Body", "width": 900, "height": 864,
          "layout": "vertical",
          "children": [ { "type": "text", "id": "t1", "name": "Title", "content": "x" } ] }
      ]
    }
  ]
}"##;

#[test]
fn finalize_reports_a_format_drift_advisory_for_a_long_form_card_board() {
    let live = load_fixture(FORMAT_DRIFT_FIXTURE);
    let tool = finalize_design_snapshot(&live);
    let outcome = tool.call(&BTreeMap::new());
    let json = match outcome {
        ToolOutcome::OkJsonWithCommand(json, _) | ToolOutcome::OkJson(json) => json,
        other => panic!("unexpected finalize outcome: {other:?}"),
    };

    let advisories = advisories_of(&json);
    let drifts: Vec<&serde_json::Value> = advisories
        .iter()
        .filter(|advisory| advisory["code"].as_str() == Some("board-format-drift"))
        .collect();
    assert_eq!(drifts.len(), 1, "exactly one format-drift advisory: {json}");
    let advisory = drifts[0];
    assert_eq!(
        advisory["nodeIds"]
            .as_array()
            .and_then(|ids| ids.first())
            .and_then(|v| v.as_str()),
        Some("root"),
        "the advisory names the board root: {json}"
    );
    let message = advisory["message"].as_str().expect("message");
    assert!(
        message.contains("1.96:1")
            && message.contains("compress content to restore 3:4")
            && message.contains("keep the long-form card if scroll-length output is acceptable"),
        "the message names the ratio and both directions: {json}"
    );
}

#[test]
fn finalize_reports_no_format_drift_for_a_regular_card_board() {
    // The P2-b full-card fixture is a regular 1080x1440 board — the
    // authored 3:4 contract, so no format-drift advisory may ride the
    // channel alongside the (also absent) void one.
    let live = load_fixture(FULL_FIXTURE);
    let tool = finalize_design_snapshot(&live);
    let outcome = tool.call(&BTreeMap::new());
    let json = match outcome {
        ToolOutcome::OkJsonWithCommand(json, _) | ToolOutcome::OkJson(json) => json,
        other => panic!("unexpected finalize outcome: {other:?}"),
    };
    let advisories = advisories_of(&json);
    let drifts: Vec<&serde_json::Value> = advisories
        .iter()
        .filter(|advisory| advisory["code"].as_str() == Some("board-format-drift"))
        .collect();
    assert!(
        drifts.is_empty(),
        "a regular 3:4 card must produce no format-drift advisory: {json}"
    );
}

// ── Shader fill findings ─────────────────────────────────────────────────────

/// A mobile frame (390x844) with one shader fill that has a 5-element uniform —
/// invalid arity, so the shader will degrade to a flat colour at paint time.
const SHADER_INVALID_FIXTURE: &str = r##"{
  "version": "1.0",
  "children": [
    {
      "type": "frame",
      "id": "root",
      "name": "App Screen",
      "width": 390,
      "height": 844,
      "layout": "vertical",
      "children": [
        { "type": "rectangle", "id": "hero", "name": "Hero", "width": 390, "height": 400,
          "fill": [{ "type": "shader", "sksl": "float f;", "uniforms": { "bad": [1.0, 2.0, 3.0, 4.0, 5.0] } }] }
      ]
    }
  ]
}"##;

#[test]
fn finalize_reports_invalid_shader_as_a_blocking_advisory() {
    let live = load_fixture(SHADER_INVALID_FIXTURE);
    let tool = finalize_design_snapshot(&live);
    let outcome = tool.call(&BTreeMap::new());
    let json = match outcome {
        ToolOutcome::OkJsonWithCommand(json, _) | ToolOutcome::OkJson(json) => json,
        other => panic!("unexpected finalize outcome: {other:?}"),
    };

    let advisories = advisories_of(&json);
    let invalid: Vec<&serde_json::Value> = advisories
        .iter()
        .filter(|advisory| advisory["code"].as_str() == Some("shader-invalid"))
        .collect();
    assert_eq!(
        invalid.len(),
        1,
        "exactly one shader-invalid advisory: {json}"
    );
    let advisory = invalid[0];
    assert_eq!(
        advisory["nodeIds"]
            .as_array()
            .and_then(|ids| ids.first())
            .and_then(|v| v.as_str()),
        Some("hero"),
        "the advisory names the node with the invalid shader: {json}"
    );
    assert!(
        advisory["message"]
            .as_str()
            .is_some_and(|m| m.contains("components") && m.contains("degrade")),
        "the message describes why the fill is invalid: {json}"
    );
    assert!(
        !complete_of(&json),
        "invalid shaders must block completion: {json}"
    );
}

/// A mobile screen (390x844) carrying three STACKED full-bleed shader layers —
/// each the exact size of the root, so their area share stays at 100% no matter
/// what the cleanup passes do to the root. The mobile full-bleed budget is 2,
/// so the third layer is over the line. Stacking (no auto-layout on the root)
/// is the real-world shape of this smell: gradient + noise + vignette overlays.
const SHADER_BUDGET_FIXTURE: &str = r##"{
  "version": "1.0",
  "children": [
    {
      "type": "frame",
      "id": "root",
      "name": "App Screen",
      "width": 390,
      "height": 844,
      "layout": "none",
      "children": [
        { "type": "rectangle", "id": "s1", "name": "Base", "width": 390, "height": 844,
          "fill": [{ "type": "shader", "sksl": "float f;" }] },
        { "type": "rectangle", "id": "s2", "name": "Noise", "width": 390, "height": 844,
          "fill": [{ "type": "shader", "sksl": "float f;" }] },
        { "type": "rectangle", "id": "s3", "name": "Vignette", "width": 390, "height": 844,
          "fill": [{ "type": "shader", "sksl": "float f;" }] }
      ]
    }
  ]
}"##;

#[test]
fn finalize_reports_shader_budget_as_informational() {
    let live = load_fixture(SHADER_BUDGET_FIXTURE);
    let tool = finalize_design_snapshot(&live);
    let outcome = tool.call(&BTreeMap::new());
    let json = match outcome {
        ToolOutcome::OkJsonWithCommand(json, _) | ToolOutcome::OkJson(json) => json,
        other => panic!("unexpected finalize outcome: {other:?}"),
    };

    let value: serde_json::Value = serde_json::from_str(&json).expect("summary json");
    let budget_hits: Vec<&serde_json::Value> = value["informational"]
        .as_array()
        .expect("informational array must be present")
        .iter()
        .filter(|entry| entry["code"].as_str() == Some("shader-budget"))
        .collect();
    assert!(
        !budget_hits.is_empty(),
        "three stacked full-bleed shaders on a mobile screen (budget 2) must \
         produce a shader-budget informational entry: {json}"
    );
    // The cost finding is advisory: it must not gate completion, must not
    // count as blocking, and must not appear among the blocking advisories.
    assert!(
        complete_of(&json),
        "informational items must not block completion: {json}"
    );
    let advisories = advisories_of(&json);
    assert_eq!(
        value["blockingAdvisoryCount"].as_u64().unwrap_or(0) as usize,
        advisories.len(),
        "blockingAdvisoryCount must match advisories.len(): {json}"
    );
    assert!(
        advisories
            .iter()
            .all(|advisory| advisory["code"].as_str() != Some("shader-budget")),
        "shader-budget must never appear as a blocking advisory: {json}"
    );
}

#[test]
fn finalize_includes_empty_informational_array_for_clean_document() {
    let live = load();
    let tool = finalize_design_snapshot(&live);
    let outcome = tool.call(&BTreeMap::new());
    let json = match outcome {
        ToolOutcome::OkJsonWithCommand(json, _) | ToolOutcome::OkJson(json) => json,
        other => panic!("unexpected finalize outcome: {other:?}"),
    };
    let value: serde_json::Value = serde_json::from_str(&json).expect("summary json");
    assert!(
        value["informational"]
            .as_array()
            .is_some_and(|a| a.is_empty()),
        "informational array must be present and empty for a clean document: {json}"
    );
}
