//! Tests for `cleanup_section_margins::unify_transparent_section_margins`
//! (DS P1.5): a root with no own margin whose transparent `fill_container`
//! sections carry drifted horizontal paddings gets ONE group margin — the
//! maximum per side. Every shape without a geometry proof of drift is left
//! alone.
//!
//! The fixture mirrors the dissected 0815 v4-pro card: a 1080x1440 portrait
//! board, root padding None, seven `fill_container` sections of which four
//! author their own horizontal padding ([0,80] / [24,80]) and three carry
//! none, so their text sits flush against the canvas left edge.

use super::*;
use crate::cleanup::run_cleanup_passes_with_summary;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::{CheckCategory, RepairSummary};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::json;

fn insert_tree(sink: &mut VecDocSink, tree: &serde_json::Value) {
    let tree: PenNode = serde_json::from_value(tree.clone()).expect("test tree json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
}

fn root_json(sink: &VecDocSink) -> serde_json::Value {
    serde_json::to_value(&sink.state.active_children()[0]).expect("serialize")
}

fn section_json(sink: &VecDocSink, id: &str) -> serde_json::Value {
    let node = op_editor_core::walkers::find_node(
        sink.state.active_children(),
        &NodeId::new(id.to_string()),
    )
    .expect("section exists");
    serde_json::to_value(node).expect("serialize")
}

fn run_pass(sink: &mut VecDocSink, root_id: &str) -> bool {
    unify_transparent_section_margins(sink, root_id)
}

/// A 1080x1440 card: root without padding, seven `fill_container` sections.
/// Four carry horizontal padding ([0,80] x2, [24,80] x2), three carry none —
/// the 0815 v4-pro card shape.
fn drifted_card() -> serde_json::Value {
    let section = |id: &str, padding: Option<serde_json::Value>| {
        let mut s = serde_json::Map::new();
        s.insert("type".into(), "frame".into());
        s.insert("id".into(), id.into());
        s.insert("name".into(), id.into());
        s.insert("width".into(), "fill_container".into());
        s.insert("height".into(), json!(120));
        s.insert("layout".into(), "vertical".into());
        if let Some(padding) = padding {
            s.insert("padding".into(), padding);
        }
        s.insert(
            "children".into(),
            json!([{ "type": "text", "id": format!("{id}-title"), "content": "标题", "fontSize": 28 }]),
        );
        serde_json::Value::Object(s)
    };
    json!({
        "type": "frame",
        "id": "card",
        "name": "知识卡片",
        "width": 1080,
        "height": 1440,
        "layout": "vertical",
        "children": [
            section("s1", Some(json!([0, 80]))),
            section("s2", Some(json!([0, 80]))),
            section("s3", Some(json!([24, 80]))),
            section("s4", Some(json!([24, 80]))),
            section("s5", None),
            section("s6", None),
            section("s7", None)
        ]
    })
}

#[test]
fn unify_section_drifted_transparent_sections_adopt_the_group_max() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &drifted_card());

    assert!(
        run_pass(&mut sink, "card"),
        "flush text must trigger the pass"
    );

    // The three unpadded sections rise to the group max horizontal (80);
    // vertical stays 0.
    for id in ["s5", "s6", "s7"] {
        assert_eq!(
            section_json(&sink, id)["padding"],
            json!([0.0, 80.0]),
            "{id} must adopt the group margin [0,80]"
        );
    }
    // Sections already at 80 horizontal keep their own vertical component.
    for id in ["s1", "s2"] {
        assert_eq!(section_json(&sink, id)["padding"], json!([0.0, 80.0]));
    }
    for id in ["s3", "s4"] {
        assert_eq!(
            section_json(&sink, id)["padding"],
            json!([24.0, 80.0]),
            "{id}: vertical 24 is not this pass's business"
        );
    }
    // The root is untouched — the sections own the margin after unification.
    assert!(root_json(&sink).get("padding").is_none());
}

#[test]
fn unify_section_a_root_that_owns_its_margin_blocks_the_pass() {
    let mut tree = drifted_card();
    tree.as_object_mut()
        .expect("object")
        .insert("padding".into(), json!([0, 40]));
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &tree);

    assert!(
        !run_pass(&mut sink, "card"),
        "a padded root owns its margin; sections must not be double-inset"
    );
    assert!(section_json(&sink, "s5").get("padding").is_none());
}

#[test]
fn unify_section_one_coloured_band_vetoes_the_whole_pass() {
    // The 0808 red line: a visible solid fill on ANY candidate section means
    // the page's bands are authored — never strip or re-margin around them.
    let mut tree = drifted_card();
    let children = tree["children"].as_array_mut().expect("children");
    children.push(json!({
        "type": "frame", "id": "band", "name": "Colored band",
        "width": "fill_container", "height": 120, "layout": "vertical",
        "fill": [{ "type": "solid", "color": "#1F2937" }],
        "children": [{ "type": "text", "id": "band-title", "content": "带", "fontSize": 28 }]
    }));
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &tree);

    assert!(!run_pass(&mut sink, "card"), "the band must veto the pass");
    assert!(section_json(&sink, "s5").get("padding").is_none());
    assert!(section_json(&sink, "band").get("padding").is_none());
}

#[test]
fn unify_section_a_transparent_fill_is_not_a_band() {
    // opacity 0 is the same as no fill: the pass may proceed.
    let mut tree = drifted_card();
    let children = tree["children"].as_array_mut().expect("children");
    children.push(json!({
        "type": "frame", "id": "ghost", "name": "Ghost band",
        "width": "fill_container", "height": 120, "layout": "vertical",
        "fill": [{ "type": "solid", "color": "#000000", "opacity": 0 }],
        "children": [{ "type": "text", "id": "ghost-title", "content": "透明", "fontSize": 28 }]
    }));
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &tree);

    assert!(run_pass(&mut sink, "card"));
    assert_eq!(section_json(&sink, "s5")["padding"], json!([0.0, 80.0]));
    assert_eq!(section_json(&sink, "ghost")["padding"], json!([0.0, 80.0]));
}

#[test]
fn unify_section_agreed_margins_are_untouched() {
    // Every section already carries [0,80]: no drift to prove.
    let mut tree = drifted_card();
    let children = tree["children"].as_array_mut().expect("children");
    for child in children.iter_mut() {
        child
            .as_object_mut()
            .expect("object")
            .insert("padding".into(), json!([0, 80]));
    }
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &tree);

    assert!(!run_pass(&mut sink, "card"));
    assert_eq!(section_json(&sink, "s1")["padding"], json!([0.0, 80.0]));
}

#[test]
fn unify_section_drifted_margins_without_flush_content_are_untouched() {
    // Paddings differ (80 vs 60) but every section keeps its content >= 24px
    // from the canvas edge — there is no geometry proof of a defect.
    let mut tree = drifted_card();
    let children = tree["children"].as_array_mut().expect("children");
    for child in children.iter_mut() {
        let id = child["id"].as_str().expect("id").to_string();
        let padding = if id == "s1" || id == "s2" {
            json!([0, 80])
        } else {
            json!([0, 60])
        };
        child
            .as_object_mut()
            .expect("object")
            .insert("padding".into(), padding);
    }
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &tree);

    assert!(!run_pass(&mut sink, "card"));
    assert_eq!(section_json(&sink, "s5")["padding"], json!([0.0, 60.0]));
}

#[test]
fn unify_section_a_mobile_screen_keeps_its_edge_to_edge_contract() {
    // Flush content on a phone is LEGAL (full-bleed rows), so the pass has
    // no proof to act on — even with drifted paddings and flush text.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "phone", "name": "Home",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "a", "name": "a", "width": "fill_container",
                  "height": 120, "layout": "vertical", "padding": [0, 24],
                  "children": [{ "type": "text", "id": "a-t", "content": "A", "fontSize": 24 }] },
                { "type": "frame", "id": "b", "name": "b", "width": "fill_container",
                  "height": 120, "layout": "vertical",
                  "children": [{ "type": "text", "id": "b-t", "content": "B", "fontSize": 24 }] },
                { "type": "frame", "id": "c", "name": "c", "width": "fill_container",
                  "height": 120, "layout": "vertical",
                  "children": [{ "type": "text", "id": "c-t", "content": "C", "fontSize": 24 }] }
            ]
        }),
    );

    assert!(
        !run_pass(&mut sink, "phone"),
        "edge-to-edge on a phone proves nothing"
    );
    assert!(section_json(&sink, "b").get("padding").is_none());
}

#[test]
fn unify_section_the_pass_is_idempotent() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &drifted_card());
    assert!(run_pass(&mut sink, "card"));
    assert!(
        !run_pass(&mut sink, "card"),
        "the second run must find the group already unified"
    );
}

#[test]
fn unify_section_driver_attributes_the_repair_to_the_checkpoint() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &drifted_card());
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "card".to_string(),
            name: "知识卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    };
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(&mut sink, &plan, &["card"], &mut summary);

    assert!(
        summary.records().iter().any(|record| {
            record.pass == "unify-section-margins" && record.category == CheckCategory::Layout
        }),
        "the pass must be mounted and checkpointed in the driver: {:?}",
        summary.records()
    );
}

// ── Convergence (DS P1.5 rework) ────────────────────────────────────────────

/// The 0815 lesion card through the FULL driver in its simplified shape: a
/// 1080x1440 card root (gap 20, NO padding) over seven `fill_container`
/// sections — four carrying horizontal 80 ([0,80] x3, [24,80] x1), three
/// unset so their text sits flush against the canvas edge.
///
/// One driver round must converge on the argued equivalent of "root
/// horizontal margin 80": the model's 80 survives as ONE uniform section
/// margin (the margin-ownership delegation the card corpus itself used), the
/// root stays unpadded, and a second round applies ZERO repairs. The chain
/// is: `unify-section-margins` raises the three unset sections to the group
/// max [0,80] (mounted BEFORE the stripper), the wrapper-inset stripper then
/// folds the [24,80] section's vertical 24 into the root gap (-> [0,80]),
/// and the slide-padding floor re-parses the post-repair layout, finds NO
/// flush content left, and stays OFF — the two measured defects (the floor
/// stomping the root to [0,48] on stale frame-level evidence, then the
/// stripper deleting every section's 80 in round two) are both gone.
fn lesion_card() -> serde_json::Value {
    let section = |id: &str, name: &str, padding: Option<serde_json::Value>| {
        let mut s = serde_json::Map::new();
        s.insert("type".into(), "frame".into());
        s.insert("id".into(), id.into());
        s.insert("name".into(), name.into());
        s.insert("width".into(), "fill_container".into());
        s.insert("height".into(), json!(120));
        s.insert("layout".into(), "vertical".into());
        if let Some(padding) = padding {
            s.insert("padding".into(), padding);
        }
        s.insert(
            "children".into(),
            json!([{ "type": "text", "id": format!("{id}-title"), "content": "标题", "fontSize": 28 }]),
        );
        serde_json::Value::Object(s)
    };
    json!({
        "type": "frame",
        "id": "card",
        "name": "知识卡片",
        "width": 1080,
        "height": 1440,
        "layout": "vertical",
        "gap": 20,
        "children": [
            section("s1", "顶部刊头区", Some(json!([0, 80]))),
            section("s2", "法则 01", Some(json!([0, 80]))),
            section("s3", "法则 02", Some(json!([0, 80]))),
            section("s4", "法则 05", Some(json!([24, 80]))),
            section("s5", "法则 03", None),
            section("s6", "法则 04", None),
            section("s7", "底部落款区", None)
        ]
    })
}

fn lesion_plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "card".to_string(),
            name: "知识卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

#[test]
fn unify_section_driver_converges_the_lesion_card_in_one_round_with_the_horizontal_floor_off() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &lesion_card());

    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(&mut sink, &lesion_plan(), &["card"], &mut summary);

    // The one round runs the whole chain: unify raises the three unset
    // sections, then the stripper normalizes the [24,80] section's vertical
    // padding against the root gap.
    assert!(
        summary
            .records()
            .iter()
            .filter(|record| record.pass == "unify-section-margins")
            .count()
            == 3,
        "the three unset sections must be raised by the unify pass: {:?}",
        summary.records()
    );
    assert!(
        summary
            .records()
            .iter()
            .any(|record| record.pass == "spacing+footer-sink"),
        "the stripper must fold the [24,80] section's vertical 24 into the root gap: {:?}",
        summary.records()
    );
    // Defect ① regression (horizontal axis): with the sections unified at 80
    // the content is no longer flush HORIZONTALLY, so the horizontal floor
    // must not take the delegated margin. DS P2-b A: the masthead text still
    // sits flush against the board TOP (every section carries zero vertical
    // padding), so the vertical floor fires and raises only TOP to 48 —
    // per-edge semantics (DS P2-c A): the content ends ~480px short of the
    // board bottom, so the bottom has no flush evidence and stays 0. Exactly
    // one slide-padding-floor record.
    assert_eq!(
        summary
            .records()
            .iter()
            .filter(|record| record.pass == "slide-padding-floor")
            .count(),
        1,
        "the vertical floor fires exactly once: {:?}",
        summary.records()
    );
    assert_eq!(
        root_json(&sink)["padding"],
        json!([48.0, 0.0, 0.0, 0.0]),
        "only TOP rises to the floor while the horizontal ownership stays \
         delegated: {}",
        root_json(&sink)
    );

    // The argued equivalent final state: the model's intended 80 survives as
    // ONE uniform horizontal section margin on all seven sections. The
    // stripper's whole-root transform renumbers node ids and may serialize
    // padding in the 4-edge form, so the sections are addressed by their
    // (stable) child order and compared per horizontal edge.
    let root = root_json(&sink);
    let children = root["children"].as_array().expect("sections array");
    assert_eq!(children.len(), 7, "all seven sections survive the round");
    for (index, child) in children.iter().enumerate() {
        let edges: Vec<f64> = child["padding"]
            .as_array()
            .expect("section padding is an array")
            .iter()
            .map(|edge| edge.as_f64().expect("numeric edge"))
            .collect();
        let (right, left) = match edges.as_slice() {
            [_vertical, horizontal] => (*horizontal, *horizontal),
            [_top, right, _bottom, left] => (*right, *left),
            other => panic!("unexpected padding shape {other:?} on section #{index}"),
        };
        assert_eq!(
            (right, left),
            (80.0, 80.0),
            "section #{index} must carry the unified group margin [0,80]: {child}"
        );
    }

    // Second round: ZERO repairs — the state is a fixed point. The root id
    // was re-allocated by the stripper's transform, so the second run is
    // driven with the CURRENT id (a stale id would silently no-op and make
    // this assertion vacuous).
    let current_root_id = sink.state.active_children()[0].id_str().to_string();
    let mut second = RepairSummary::default();
    run_cleanup_passes_with_summary(&mut sink, &lesion_plan(), &[&current_root_id], &mut second);
    assert_eq!(
        second.total_repairs(),
        0,
        "the converged state must need no further repairs: {:?}",
        second.records()
    );
}
