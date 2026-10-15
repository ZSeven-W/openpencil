//! End-to-end coverage for the quality tally the cleanup driver produces —
//! the counting layer added on top of `run_cleanup_passes`, NOT the passes
//! themselves. Two invariants matter here and nowhere else:
//!
//! 1. Reaching a checkpoint records the category as CHECKED, so a document
//!    with nothing wrong still yields a positive credential.
//! 2. A repair the passes actually make shows up as a non-zero count under
//!    the category that made it — the number is measured, not asserted.

use crate::cleanup::run_cleanup_passes_with_summary;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::{CheckCategory, RepairSummary};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::json;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
        palette: None,
    }
}

fn run_with_summary(tree: serde_json::Value) -> (VecDocSink, RepairSummary) {
    let mut sink = VecDocSink::new();
    let node: PenNode = serde_json::from_value(tree).expect("fixture json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(&mut sink, &plan(), &[&root_id], &mut summary);
    (sink, summary)
}

#[test]
fn a_clean_document_still_reports_every_category_as_checked() {
    let (_sink, summary) = run_with_summary(json!({
        "type": "frame",
        "id": "root",
        "name": "Simple Page",
        "width": 1200,
        "height": 400,
        "layout": "vertical",
        "children": [{
            "type": "text",
            "id": "title",
            "role": "heading",
            "content": "Hello",
            "fontSize": 32,
            "fontWeight": 700
        }]
    }));

    assert!(
        !summary.is_empty(),
        "the passes ran, so the credential must not be suppressed"
    );
    assert_eq!(
        summary.checked(),
        CheckCategory::ALL.to_vec(),
        "every checkpoint in the driver is reached on any non-empty root"
    );
}

#[test]
fn no_root_ids_leaves_nothing_vouched_for_per_root() {
    // Whole-document passes still run (and still checkpoint), but the
    // per-root loop is skipped entirely. What matters is that the summary is
    // never fabricated: it reports only categories whose checkpoints ran.
    let mut sink = VecDocSink::new();
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(&mut sink, &plan(), &[], &mut summary);

    for category in summary.checked() {
        assert_eq!(
            summary.repairs_for(category),
            0,
            "an empty document cannot have repaired anything"
        );
    }
}

#[test]
fn an_over_bold_screen_counts_its_repairs_under_hierarchy() {
    // Same fixture as `cleanup_tests::run_cleanup_passes_repairs_overbold_
    // text_hierarchy`, which proves the pass fires; here we assert the tally
    // sees it, and attributes it to the right category.
    let (sink, summary) = run_with_summary(json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "width": 390,
        "height": 844,
        "children": [
            { "type": "text", "id": "title", "role": "heading",
              "content": "Popular Restaurants", "width": 320, "height": 40,
              "fontSize": 30, "fontWeight": 800 },
            { "type": "text", "id": "subtitle", "role": "body-text",
              "content": "Fresh Brooklyn favorites, delivered fast.",
              "width": 320, "height": 22, "fontSize": 16, "fontWeight": 800 },
            { "type": "text", "id": "placeholder", "name": "Placeholder",
              "content": "Search restaurants or dishes", "width": 280,
              "height": 24, "fontSize": 17, "fontWeight": 800 },
            { "type": "text", "id": "metadata", "role": "caption",
              "content": "20-30 min", "width": 100, "height": 18,
              "fontSize": 14, "fontWeight": 800 }
        ]
    }));

    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetNodeFontWeight {
                font_weight: 400,
                ..
            }
        )),
        "precondition: the hierarchy pass must actually fire on this fixture"
    );
    assert!(
        summary.repairs_for(CheckCategory::Hierarchy) > 0,
        "the demotions the pass applied must be counted under hierarchy: {summary:?}"
    );
    assert!(
        summary.total_repairs() >= summary.repairs_for(CheckCategory::Hierarchy),
        "the headline total can never be smaller than one of its parts"
    );
}

#[test]
fn interaction_backfill_edits_are_counted_as_structure_repairs() {
    let nodes: Vec<PenNode> = serde_json::from_value(json!([
        {
            "type": "frame", "id": "entry", "name": "Discover", "screen": "/",
            "x": 0, "y": 0, "width": 390, "height": 844, "layout": "none",
            "children": [{
                "type": "frame", "id": "row", "x": 20, "y": 180,
                "width": 350, "height": 170, "layout": "horizontal",
                "children": [
                    {
                        "type": "frame", "id": "card-a", "width": 100, "height": 150,
                        "layout": "vertical", "children": [
                            {"type":"image","id":"image-a","width":100,"height":90,
                             "src":"https://example.invalid/a.png"},
                            {"type":"text","id":"title-a","content":"A","width":100,"height":20}
                        ]
                    },
                    {
                        "type": "frame", "id": "card-b", "width": 100, "height": 150,
                        "layout": "vertical", "children": [
                            {"type":"image","id":"image-b","width":100,"height":90,
                             "src":"https://example.invalid/b.png"},
                            {"type":"text","id":"title-b","content":"B","width":100,"height":20}
                        ]
                    }
                ]
            }]
        },
        {
            "type": "frame", "id": "detail", "name": "Movie Detail",
            "screen": "/detail", "x": 450, "y": 0, "width": 390, "height": 844,
            "layout": "none", "children": [
                {
                    "type": "frame", "id": "back", "x": 24, "y": 80,
                    "width": 44, "height": 44, "layout": "none", "children": [{
                        "type":"icon_font","id":"back-icon","x":12,"y":12,
                        "width":20,"height":20,"iconFontName":"arrow-left"
                    }]
                },
                {"type":"frame","id":"detail-content","x":0,"y":160,
                 "width":390,"height":600,"children":[]}
            ]
        }
    ]))
    .expect("fixture nodes");
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes,
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_ids = sink
        .state
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect::<Vec<_>>();
    let root_refs = root_ids.iter().map(String::as_str).collect::<Vec<_>>();
    sink.applied.clear();
    let mut summary = RepairSummary::default();

    run_cleanup_passes_with_summary(&mut sink, &plan(), &root_refs, &mut summary);

    let interaction_patches = sink
        .applied
        .iter()
        .filter(|command| {
            matches!(
                command,
                EditorCommand::PatchNodeData { patch_json, .. }
                    if patch_json.contains(r#""pop":null"#)
                        || patch_json.contains(r#""push":"\"/detail\"""#)
            )
        })
        .count();
    assert_eq!(
        interaction_patches, 3,
        "one back frame and two cards must be persisted"
    );
    assert!(
        summary.repairs_for(CheckCategory::Structure) >= interaction_patches,
        "the quality credential must count every interaction patch: {summary:?}"
    );
}

#[test]
fn the_tally_never_exceeds_the_edits_the_sink_actually_took() {
    // The credential's number must be defensible against the document: it
    // counts accepted applies, so it can never claim more repairs than the
    // sink recorded.
    let (sink, summary) = run_with_summary(json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "width": 390,
        "height": 844,
        "children": [
            { "type": "text", "id": "a", "role": "body-text", "content": "One",
              "width": 320, "height": 22, "fontSize": 16, "fontWeight": 800 },
            { "type": "text", "id": "b", "role": "caption", "content": "Two",
              "width": 100, "height": 18, "fontSize": 14, "fontWeight": 800 }
        ]
    }));

    assert!(
        summary.total_repairs() <= sink.applied.len(),
        "counted {} repairs but the sink only took {} edits",
        summary.total_repairs(),
        sink.applied.len()
    );
}

#[test]
fn the_counts_are_the_itemized_records_grouped_and_nothing_else() {
    // The one invariant that keeps the credential honest end to end: the
    // number the user is shown and the list they expand are the SAME data.
    // Maintaining them separately is how a "41 repairs" headline ends up over
    // 39 lines with nobody able to say which two are missing.
    let (_sink, summary) = run_with_summary(json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "width": 390,
        "height": 844,
        "children": [
            { "type": "text", "id": "title", "role": "heading",
              "content": "Popular Restaurants", "width": 320, "height": 40,
              "fontSize": 30, "fontWeight": 800 },
            { "type": "text", "id": "subtitle", "role": "body-text",
              "content": "Fresh Brooklyn favorites, delivered fast.",
              "width": 320, "height": 22, "fontSize": 16, "fontWeight": 800 },
            { "type": "text", "id": "placeholder", "name": "Placeholder",
              "content": "Search restaurants or dishes", "width": 280,
              "height": 24, "fontSize": 17, "fontWeight": 800 },
            { "type": "text", "id": "metadata", "role": "caption",
              "content": "20-30 min", "width": 100, "height": 18,
              "fontSize": 14, "fontWeight": 800 }
        ]
    }));

    assert_eq!(
        summary.total_repairs(),
        summary.records().len(),
        "the headline total is the record list's length, not a second tally"
    );
    for category in CheckCategory::ALL {
        assert_eq!(
            summary.repairs_for(category),
            summary
                .records()
                .iter()
                .filter(|record| record.category == category)
                .count(),
            "the per-category count for {category:?} must be its records grouped"
        );
    }
    let repaired_total: usize = summary.repaired().iter().map(|(_, count)| count).sum();
    assert_eq!(
        repaired_total,
        summary.records().len(),
        "the breakdown must account for every record, with none double-counted"
    );
}

#[test]
fn every_record_names_the_pass_the_node_and_the_change() {
    // "41 auto-repair(s) applied" with no way to see WHAT moved was the whole
    // complaint. Each record must carry an attributed pass, a real target,
    // and a field-level description — an empty one is a repair nobody can
    // audit.
    let (_sink, summary) = run_with_summary(json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "width": 390,
        "height": 844,
        "children": [
            { "type": "text", "id": "title", "role": "heading",
              "content": "Popular Restaurants", "width": 320, "height": 40,
              "fontSize": 30, "fontWeight": 800 },
            { "type": "text", "id": "subtitle", "role": "body-text",
              "content": "Fresh Brooklyn favorites, delivered fast.",
              "width": 320, "height": 22, "fontSize": 16, "fontWeight": 800 },
            { "type": "text", "id": "placeholder", "name": "Placeholder",
              "content": "Search restaurants or dishes", "width": 280,
              "height": 24, "fontSize": 17, "fontWeight": 800 },
            { "type": "text", "id": "metadata", "role": "caption",
              "content": "20-30 min", "width": 100, "height": 18,
              "fontSize": 14, "fontWeight": 800 }
        ]
    }));

    assert!(
        !summary.records().is_empty(),
        "precondition: this fixture must provoke repairs"
    );
    for record in summary.records() {
        assert!(!record.pass.is_empty(), "unattributed record: {record:?}");
        assert_ne!(
            record.pass, "unattributed",
            "a real cleanup run must never fall back to the count-only shim: {record:?}"
        );
        assert!(!record.detail.is_empty(), "undescribed record: {record:?}");
        assert!(
            record.line().contains(&record.detail),
            "the rendered line must carry the change it describes: {record:?}"
        );
    }
    let hierarchy: Vec<String> = summary
        .records()
        .iter()
        .filter(|record| record.category == CheckCategory::Hierarchy)
        .map(|record| record.detail.clone())
        .collect();
    assert!(
        hierarchy
            .iter()
            .any(|detail| detail == "fontWeight 800 → 400"),
        "the hierarchy demotion must be itemized with before → after: {hierarchy:?}"
    );
}

#[test]
fn the_double_inset_stripper_reports_which_section_lost_which_padding() {
    // The incident this whole itemized-record line exists for: a run reported
    // "41 auto-repair(s) applied — layout 36" and the user could see their
    // section rhythm had been flattened but not by what. 36 of those came from
    // `strip_wrapper_double_inset`. Pinned here as the worked example — the
    // record must name the SECTION and the padding it lost, because "padding
    // dropped on Hero Section" is the sentence that makes the repair
    // disputable instead of mysterious.
    let (_sink, summary) = run_with_summary(json!({
        "type": "frame",
        "id": "root",
        "name": "Home Screen",
        "width": 390,
        "height": 844,
        "layout": "vertical",
        "gap": 20,
        "padding": [24, 20, 24, 20],
        "children": [{
            "type": "frame",
            "id": "hero",
            "name": "Hero Section",
            "layout": "vertical",
            "gap": 8,
            "padding": [24, 20, 24, 20],
            "children": [
                { "type": "text", "id": "t1", "role": "heading", "content": "Good morning",
                  "width": 300, "height": 32, "fontSize": 24, "fontWeight": 700 },
                { "type": "text", "id": "t2", "role": "body-text", "content": "Three tasks today",
                  "width": 300, "height": 20, "fontSize": 15, "fontWeight": 400 }
            ]
        }]
    }));

    let padding_records: Vec<String> = summary
        .records()
        .iter()
        .filter(|record| record.detail.contains("padding"))
        .map(|record| record.line())
        .collect();
    assert!(
        padding_records
            .iter()
            .any(|line| line.contains("Hero Section")
                && line.contains("padding [24,20,24,20] → (unset)")),
        "the stripped section and the inset it lost must both be named: {padding_records:?}"
    );
    assert!(
        padding_records
            .iter()
            .any(|line| line.starts_with("layout · spacing+footer-sink")),
        "the repair must be attributed to the pass group that made it: {padding_records:?}"
    );
}
