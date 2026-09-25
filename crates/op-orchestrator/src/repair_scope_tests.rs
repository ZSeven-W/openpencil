//! Append-mode scope: contract checks cover the whole target root, intent
//! rewrites stay inside what the run appended.
//!
//! One fixture, two calls — the halves are only meaningful against each other.
//! The pre-existing half carries `0808-gm-2.op`'s starved-row defect (a rigid
//! `fill_container` row squeezed below its content width until its columns
//! spill across the sibling); the appended half carries a transparent padded
//! wrapper, which is what the intent-tier `strip_wrapper_double_inset` removes.

use super::*;
use op_editor_core::EditorState;
use serde_json::json;

/// `page` (the append target)
/// ├── `preexisting` — starved rigid row: CONTRACT defect, not this run's work
/// └── `appended`    — padded column + transparent padded wrapper: INTENT bait
fn state() -> EditorState {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": [{
            "type":"frame","id":"page","name":"Page","width":1200,"height":900,
            "layout":"vertical","gap":24,
            "fill":[{"type":"solid","color":"#FFFFFF"}],
            "children":[
                {"type":"frame","id":"preexisting","name":"Pre-existing Row","layout":"horizontal",
                 "width":"fill_container","gap":40,
                 "children":[
                    {"type":"frame","id":"brand","layout":"vertical","width":280,
                     "children":[{"type":"text","id":"brandt","content":"Kilnform",
                                  "fill":[{"type":"solid","color":"#111111"}]}]},
                    {"type":"frame","id":"nav","name":"Nav Columns","layout":"horizontal",
                     "width":"fill_container","gap":48,
                     "children":[
                        {"type":"frame","id":"c1","layout":"vertical","width":96,
                         "children":[{"type":"text","id":"c1t","content":"Product",
                                      "fill":[{"type":"solid","color":"#111111"}]}]},
                        {"type":"frame","id":"c2","layout":"vertical","width":96,
                         "children":[{"type":"text","id":"c2t","content":"Company",
                                      "fill":[{"type":"solid","color":"#111111"}]}]},
                        {"type":"frame","id":"c3","layout":"vertical","width":96,
                         "children":[{"type":"text","id":"c3t","content":"Legal",
                                      "fill":[{"type":"solid","color":"#111111"}]}]},
                        {"type":"frame","id":"c4","layout":"vertical","width":96,
                         "children":[{"type":"text","id":"c4t","content":"Terms",
                                      "fill":[{"type":"solid","color":"#111111"}]}]}
                     ]},
                    {"type":"frame","id":"news","layout":"vertical",
                     "width":"fill_container",
                     "children":[{"type":"text","id":"newst","content":"Newsletter",
                                  "fill":[{"type":"solid","color":"#111111"}]}]}
                 ]},
                {"type":"frame","id":"appended","name":"Appended Section","layout":"vertical",
                 "width":"fill_container","gap":20,"padding":[32, 40],
                 "children":[
                    {"type":"frame","id":"wrap","name":"Wrapper","layout":"vertical","padding":[16, 24],
                     "children":[
                        {"type":"frame","id":"card","layout":"vertical","padding":24,
                         "fill":[{"type":"solid","color":"#F1F1F1"}],
                         "children":[{"type":"text","id":"cardt","content":"Card",
                                      "fill":[{"type":"solid","color":"#111111"}]}]}
                     ]}
                 ]}
            ]
        }]
    }))
    .expect("valid doc");
    EditorState::from_document(doc)
}

fn plan() -> crate::plan::OrchestratorPlan {
    crate::plan::OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: "page".to_string(),
            name: "Page".to_string(),
            width: 1200.0,
            height: 900.0,
            layout: Some("vertical".to_string()),
            gap: Some(24.0),
            padding: None,
            fill: None,
        },
        subtasks: Vec::new(),
        style_guide_name: None,
    }
}

/// Look nodes up by NAME. `ReplaceSubtree` (every root-transform pass) hands
/// the rewritten root a fresh id, so an id captured from the fixture does not
/// survive the ordinary path.
fn node(state: &EditorState, name: &str) -> serde_json::Value {
    fn walk(v: &serde_json::Value, name: &str) -> Option<serde_json::Value> {
        if v.get("name").and_then(serde_json::Value::as_str) == Some(name) {
            return Some(v.clone());
        }
        for child in v.get("children")?.as_array()? {
            if let Some(hit) = walk(child, name) {
                return Some(hit);
            }
        }
        None
    }
    for root in state.active_children() {
        let v = serde_json::to_value(root).expect("serializes");
        if let Some(hit) = walk(&v, name) {
            return hit;
        }
    }
    panic!("{name} exists")
}

fn run(inserted: &[&str]) -> (EditorState, RepairSummary) {
    let mut state = state();
    let mut summary = RepairSummary::default();
    {
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        finalize_appended_design(&mut sink, &plan(), inserted, &["page"], &mut summary);
    }
    (state, summary)
}

#[test]
fn contract_reaches_pre_existing_content_intent_does_not() {
    // The zero-inserted-roots case — the one that shipped `0808-gm-2.op`'s
    // overlapping footer. Contract must still cover the target root.
    let (state, summary) = run(&[]);

    assert_eq!(
        node(&state, "Nav Columns")["width"],
        json!("fit_content"),
        "contract-tier geometry repair reached the pre-existing starved row"
    );
    assert_eq!(
        node(&state, "Wrapper")["padding"],
        json!([16.0, 24.0]),
        "intent-tier passes ran on nothing, so untouched content stays authored"
    );
    assert!(
        summary
            .notes()
            .iter()
            .any(|n| n.contains("0 inserted roots")),
        "the narrowed scope is recorded, not silent: {:?}",
        summary.notes()
    );
}

#[test]
fn the_contract_sweep_covers_the_target_root_whatever_was_appended() {
    // With a real appended root the sweep must still reach the PRE-EXISTING
    // half — the scope widening is unconditional, not a fallback for the
    // empty case.
    let (state, _) = run(&["appended"]);
    assert_eq!(node(&state, "Nav Columns")["width"], json!("fit_content"));
}

#[test]
fn root_transform_passes_do_not_reach_a_nested_appended_root() {
    // CHARACTERIZATION, not an endorsement. `cleanup_root_transform::
    // apply_root_transform` looks its root up with
    // `active_children().iter().position(...)` — TOP-LEVEL ONLY — while an
    // append run's `inserted_root_ids` are subtrees UNDER the target frame.
    // So the whole root-transform family (`strip_wrapper_double_inset`,
    // `regroup_flat_table_rows`, `reshape_sidebar_to_app_shell`,
    // `wrap_ring_fragments`, `sink_main_axis_distribution`, …) no-ops on an
    // appended root today; it only logs `cleanup: root id not found`.
    //
    // This is a SECOND scope defect, reported separately — pinned here so the
    // day someone teaches `apply_root_transform` to find a nested root, this
    // test fails and the fix gets noticed instead of silently changing append
    // behaviour.
    let (state, _) = run(&["appended"]);
    assert_eq!(
        node(&state, "Wrapper")["padding"],
        json!([16.0, 24.0]),
        "today the appended subtree's wrapper padding survives untouched"
    );
}

#[test]
fn the_scope_note_names_which_half_was_narrowed() {
    let (_, summary) = run(&["appended"]);
    assert!(
        summary
            .notes()
            .iter()
            .any(|n| n.contains("intent-tier passes scoped to the 1 appended root")),
        "{:?}",
        summary.notes()
    );
}

#[test]
fn the_contract_sweep_reports_its_categories() {
    // Overflow is checkpointed ONLY inside the per-root block, so its presence
    // is the credential-level proof that the block was reached at all — the
    // signature the gm-2 investigation used to tell "skipped" from "clean".
    let (_, summary) = run(&[]);
    assert!(
        summary.checked().contains(&CheckCategory::Overflow),
        "{:?}",
        summary.checked()
    );
}

#[test]
fn the_ordinary_path_is_untouched() {
    // Non-append runs go through `finalize_design_with_summary` directly. Both
    // tiers must still cover the whole page there — this pins that the change
    // added a scope, it did not move the main path onto one.
    let mut state = state();
    let mut summary = RepairSummary::default();
    {
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        crate::cleanup::finalize_design_with_summary(&mut sink, &plan(), &["page"], &mut summary);
    }
    assert_eq!(
        node(&state, "Nav Columns")["width"],
        json!("fit_content"),
        "contract"
    );
    // Both axes qualify here (the column pads 40 horizontally and gaps 24,
    // and 16 sits inside the gap's duplicate window), so the whole padding
    // key goes — the intent tier reached it.
    assert!(
        node(&state, "Wrapper")["padding"].is_null(),
        "intent-tier strip reached the wrapper on the unscoped path"
    );
    assert!(
        summary.notes().iter().all(|n| !n.contains("scoped to")),
        "no scope note on the unscoped path: {:?}",
        summary.notes()
    );
}

#[test]
fn an_authored_import_gets_contract_checks_only() {
    // The appended half's transparent padded wrapper is exactly what the
    // intent tier strips; an authored import must keep it.
    let mut state = state();
    let before = serde_json::to_value(
        op_editor_core::walkers::find_node(
            state.active_children(),
            &op_editor_core::NodeId::new("appended"),
        )
        .unwrap(),
    )
    .unwrap();
    let summary = crate::repair_scope::finalize_authored_import(&mut state, "acme.example");
    assert!(summary
        .notes()
        .iter()
        .any(|note| note.contains("authored import: acme.example")));
    let after = serde_json::to_value(
        op_editor_core::walkers::find_node(
            state.active_children(),
            &op_editor_core::NodeId::new("appended"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        before["children"][0]["padding"], after["children"][0]["padding"],
        "intent-tier wrapper inset kept"
    );
    // The contract tier ran: its categories are on the ledger.
    assert!(!summary.checked().is_empty());
}
