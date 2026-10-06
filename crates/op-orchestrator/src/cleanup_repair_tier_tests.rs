//! What the repair-tier gate does to a real cleanup run.
//!
//! The unit tests beside `repair_tier` / `template_provenance` prove the
//! classification and the signal; these drive the whole driver
//! (`run_cleanup_passes_with_summary`) over the same tree twice — once with
//! template provenance and once without — because that is the only place the
//! gate can be shown to be *reached*. Every assertion is paired: what the
//! intent tier must NOT do to authored input, and what it still does to
//! ordinary generated output, so a gate that simply disabled the pass
//! everywhere fails as loudly as a gate that never fires.
//!
//! `InsertSubtree` remaps node ids, and the structural passes swap whole roots
//! for fresh ids, so everything here is looked up BY NAME.

use crate::cleanup::run_cleanup_passes_with_summary;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::RepairSummary;
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Screen".into(),
            width: 390.0,
            height: 844.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

/// A phone-width board, deliberately: `strip_wrapper_double_inset`'s own
/// scrolling-page guard (`design_type::DesignForm::Page`) protects a 1200-wide
/// marketing page's section rhythm already, so a page fixture would pass these
/// tests for the wrong reason. On a 390 board the tier gate is the only thing
/// standing between the authored insets and the stripper — which is exactly
/// what `removing_the_gate_strips_the_authored_padding` below proves.
///
/// The insets are 24px against a 20px column gap. That is inside the band the
/// stripper reads as a duplicate of the gap (`GAP_DUPLICATE_FACTOR`), so the
/// pass genuinely fires here; the deeper 40–80px insets of `0808-gm-1` are
/// already refused by that guard and would not exercise this one.
fn authored_board() -> Value {
    json!({
        "type": "frame", "id": "board", "name": "Cover",
        "width": 390, "height": 844,
        "layout": "vertical", "gap": 20,
        "fill": [{"type": "solid", "color": "#FFFFFF"}],
        "children": [
            {
                "type": "frame", "id": "band", "name": "Feature band",
                "layout": "vertical", "padding": [24, 32], "gap": 12,
                "children": [
                    {"type": "text", "id": "band-title", "name": "Band title",
                     "role": "heading", "content": "Designed on purpose",
                     "fontSize": 28,
                     "fill": [{"type": "solid", "color": "#0A0A0A"}]}
                ]
            },
            {
                "type": "frame", "id": "note", "name": "Note",
                "layout": "vertical", "padding": [24, 32], "gap": 12,
                "children": [
                    {"type": "text", "id": "note-body", "name": "Note body",
                     "role": "body",
                     "content": "Room around this block is the composition.",
                     "fontSize": 15,
                     "fill": [{"type": "solid", "color": "#0A0A0A"}]}
                ]
            }
        ]
    })
}

/// The variable table an appended `slide-deck` leaves in the document — the
/// tree-side half of the provenance signal, taken from the real append path.
fn deck_variable_names() -> Vec<String> {
    let deck = op_editor_core::scene_template_catalog::scene_template_by_id("slide-deck")
        .expect("the deck template ships");
    op_editor_core::scene_template_append::template_boards(deck.document(), &deck.id)
        .expect("the shipped template parses")
        .variables
        .into_keys()
        .collect()
}

/// Run the driver over `tree`. `template` seeds the document with a real
/// appended template's variable table, which is what the provenance judge
/// reads; without it the same tree is ordinary generated output.
fn run(tree: Value, template: bool) -> (Value, RepairSummary) {
    let mut sink = VecDocSink::new();
    if template {
        let mut table = std::collections::BTreeMap::new();
        for name in deck_variable_names() {
            table.insert(
                name,
                serde_json::from_value(json!({"type": "color", "value": "#0A0A0A"}))
                    .expect("variable fixture"),
            );
        }
        // A generation into a template-provenance document still emits its own
        // design-system tokens; the contract-tier text-contrast repair re-points
        // at those, so leaving them out would make it no-op for a reason that
        // has nothing to do with the tier gate.
        for (name, value) in [("--card", "#FFFFFF"), ("--foreground", "#0F172A")] {
            table.insert(
                name.to_string(),
                serde_json::from_value(json!({"type": "color", "value": value}))
                    .expect("variable fixture"),
            );
        }
        sink.state.doc.variables = Some(table);
    }
    let node: PenNode = serde_json::from_value(tree).expect("fixture json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(&mut sink, &plan(), &[&root_id], &mut summary);
    let out = serde_json::to_value(&sink.state.active_children()[0]).expect("serialize");
    (out, summary)
}

fn find_by_name<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    if node.get("name").and_then(Value::as_str) == Some(name) {
        return Some(node);
    }
    node.get("children")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|child| find_by_name(child, name))
}

fn named<'a>(root: &'a Value, name: &str) -> &'a Value {
    find_by_name(root, name).unwrap_or_else(|| panic!("`{name}` survived cleanup"))
}

/// `(top, bottom)` of whatever padding shorthand the node carries.
fn vertical_padding(node: &Value) -> (f64, f64) {
    match node.get("padding") {
        Some(Value::Number(n)) => {
            let v = n.as_f64().unwrap_or(0.0);
            (v, v)
        }
        Some(Value::Array(a)) => {
            let at = |i: usize| a.get(i).and_then(Value::as_f64).unwrap_or(0.0);
            match a.len() {
                2 => (at(0), at(0)),
                4 => (at(0), at(2)),
                _ => (0.0, 0.0),
            }
        }
        _ => (0.0, 0.0),
    }
}

// ── the intent tier leaves an authored design alone ────────────────────────

#[test]
fn authored_spacing_survives_a_full_cleanup_run() {
    let (out, _summary) = run(authored_board(), true);

    for section in ["Feature band", "Note"] {
        assert_eq!(
            vertical_padding(named(&out, section)),
            (24.0, 24.0),
            "`{section}`'s authored inset is the composition, not a double inset"
        );
    }
}

/// The paired half. Without it, a gate that disabled the pass for every
/// document would pass every assertion above.
#[test]
fn the_same_tree_without_provenance_is_repaired_as_before() {
    let (out, _summary) = run(authored_board(), false);

    let (top, bottom) = vertical_padding(named(&out, "Note"));
    assert_eq!(
        (top, bottom),
        (0.0, 0.0),
        "ordinary generated output still gets the double-inset strip"
    );
}

/// The red check, mechanised. Removing the gate amounts to calling the pass
/// on the same tree, and it must strip the very padding the gated run kept —
/// otherwise the two tests above are agreeing about a pass that never fires.
#[test]
fn removing_the_gate_strips_the_authored_padding() {
    let mut root: PenNode = serde_json::from_value(authored_board()).expect("fixture");
    assert!(
        crate::spacing_repair::strip_wrapper_double_inset(&mut root),
        "ungated, the pass fires on this exact tree"
    );
    let out = serde_json::to_value(&root).expect("serialize");
    assert_eq!(
        vertical_padding(named(&out, "Note")),
        (0.0, 0.0),
        "the red check must actually go red"
    );
}

// ── the contract tier still runs on authored input ─────────────────────────

#[test]
fn invisible_text_is_still_repaired_for_authored_input() {
    // Near-black text on a near-black card: the text is not styled, it is
    // missing. No author meant that, so provenance buys it nothing — the
    // contract tier repairs it exactly as it would anywhere else.
    let mut tree = authored_board();
    tree["children"]
        .as_array_mut()
        .expect("children")
        .push(json!({
            "type": "frame", "id": "card", "name": "Dark card",
            "layout": "vertical", "padding": 20,
            "fill": [{"type": "solid", "color": "#0A0A0A"}],
            "children": [
                {"type": "text", "id": "ghost", "name": "Ghost line", "role": "body",
                 "content": "Invisible against its own card", "fontSize": 15,
                 "fill": [{"type": "solid", "color": "#111111"}]}
            ]
        }));

    let (out, summary) = run(tree, true);

    let fill = &named(&out, "Ghost line")["fill"];
    assert_ne!(
        fill[0]["color"],
        json!("#111111"),
        "the contract tier must re-point invisible text even for authored input: {fill}"
    );
    assert!(
        summary.total_repairs() > 0,
        "the contract tier applied edits even though the intent tier stood down"
    );
    // …and the intent tier still stood down on the same run.
    assert_eq!(vertical_padding(named(&out, "Note")), (24.0, 24.0));
}

// ── the ledger says the skip was a decision ────────────────────────────────

#[test]
fn the_ledger_records_the_skip_once_and_names_the_template() {
    let (_out, summary) = run(authored_board(), true);

    assert_eq!(summary.notes().len(), 1, "{:?}", summary.notes());
    let note = &summary.notes()[0];
    assert!(note.contains("intent-tier passes skipped"), "{note}");
    assert!(note.contains("slide-deck"), "{note}");
    assert!(
        summary.total_repairs() == summary.records().len(),
        "a note must never be counted as a repair"
    );
}

#[test]
fn an_ordinary_run_says_nothing_about_tiers() {
    let (_out, summary) = run(authored_board(), false);
    assert!(summary.notes().is_empty(), "{:?}", summary.notes());
}
