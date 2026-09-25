//! The repair-tier gate on the agentic-loop finalize path.
//!
//! The loop has no plan, so a provenance signal carried on the request could
//! never reach it — which is why [`crate::repair_tier::RepairTierPolicy`] is
//! resolved from the document. These tests are what proves it arrives: the
//! same forest is finalized twice, once with template provenance and once
//! without, and the intent-tier passes only stand down for the first.
//!
//! The surface passes (`fix_structural_wrapper_transparency`,
//! `fix_surface_color_discipline`) run HERE and not in the cleanup driver, so
//! this is the only place their gate can be observed end to end.

use super::apply_loop_finalize;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use serde_json::{json, Value};

/// `InsertSubtree` remaps ids, so everything here is looked up by name.
fn state_with_forest(nodes: Value, template: bool) -> EditorState {
    let parsed: Vec<jian_ops_schema::node::PenNode> =
        serde_json::from_value(nodes).expect("valid PenNode forest");
    let mut state = EditorState::new();
    if template {
        state.editor_ui.scene_template_center.generate_basis = Some("slide-deck".to_string());
    }
    state.apply(EditorCommand::InsertSubtree {
        nodes: parsed,
        parent_id: NodeId::NONE,
        page_id: None,
    });
    state
}

fn find_by_name<'a>(
    nodes: &'a [jian_ops_schema::node::PenNode],
    name: &str,
) -> Option<&'a jian_ops_schema::node::PenNode> {
    for node in nodes {
        if node.base().name.as_deref() == Some(name) {
            return Some(node);
        }
        if let Some(children) = node.children() {
            if let Some(hit) = find_by_name(children, name) {
                return Some(hit);
            }
        }
    }
    None
}

fn fill_of(state: &EditorState, name: &str) -> Option<Value> {
    let node = find_by_name(state.active_children(), name)?;
    serde_json::to_value(node)
        .ok()?
        .get("fill")
        .cloned()
        .or(Some(Value::Null))
}

/// The `loop_finalize_resolves_roles_and_strips_white_wrapper` fixture: a
/// `section`-roled wrapper carrying a white fill, which the intent-tier
/// structural-wrapper-transparency pass strips.
fn forest_with_white_section() -> Value {
    json!([
        {
            "type": "frame", "id": "header", "name": "Header",
            "x": 0, "y": 0, "width": 1200, "height": 64,
            "children": [
                {"type": "text", "id": "logo", "name": "Logo", "content": "Acme"}
            ]
        },
        {
            "type": "frame", "id": "content", "name": "Content",
            "x": 0, "y": 64, "width": 1200, "height": 400,
            "children": [
                {
                    "type": "frame", "id": "wrapper", "name": "Inner Section",
                    "role": "section",
                    "x": 0, "y": 0, "width": 1200, "height": 400,
                    "fill": [{"type": "solid", "color": "#FFFFFF"}],
                    "children": [
                        {"type": "text", "id": "body", "name": "Body", "content": "Hello"}
                    ]
                }
            ]
        }
    ])
}

#[test]
fn an_authored_section_surface_survives_loop_finalize() {
    let mut state = state_with_forest(forest_with_white_section(), true);
    apply_loop_finalize(&mut state);

    assert_eq!(
        fill_of(&state, "Inner Section"),
        Some(json!([{"type": "solid", "color": "#FFFFFF"}])),
        "a template's own card surface is authored truth, not a stray white fill"
    );
}

/// The paired half — and simultaneously the red check for the test above: the
/// gate is the only difference between the two runs, so a gate that never
/// fired would fail the first and a gate that always fired would fail this.
#[test]
fn the_same_forest_without_provenance_is_stripped_as_before() {
    let mut state = state_with_forest(forest_with_white_section(), false);
    apply_loop_finalize(&mut state);

    assert_eq!(
        fill_of(&state, "Inner Section"),
        Some(json!([])),
        "ordinary loop output still gets the white structural wrapper stripped"
    );
}

/// The tier gate must not disable the contract-tier work the same finalize
/// does — role resolution and the whole-doc cleanup still run.
#[test]
fn contract_work_still_runs_under_template_provenance() {
    let mut state = state_with_forest(forest_with_white_section(), true);
    apply_loop_finalize(&mut state);

    let header = find_by_name(state.active_children(), "Header").expect("Header survives");
    assert_eq!(
        header.base().role.as_deref(),
        Some("navbar"),
        "role inference is not an intent-tier pass and must still run"
    );
}

/// A document imported from a live website is authored input too: a later AI
/// edit's loop finalize must keep the site's own surfaces, exactly as it does
/// for a template (the paired no-provenance run above is the red check).
#[test]
fn an_imported_site_surface_survives_a_later_loop_finalize() {
    let mut state = state_with_forest(forest_with_white_section(), false);
    state.editor_ui.home.imported_from = Some("https://acme.example/".to_string());
    apply_loop_finalize(&mut state);

    assert_eq!(
        fill_of(&state, "Inner Section"),
        Some(json!([{"type": "solid", "color": "#FFFFFF"}])),
        "an imported site's own section surface is the author's, not a stray fill"
    );
}
