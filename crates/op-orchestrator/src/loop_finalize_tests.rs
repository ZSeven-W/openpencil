//! Tests for the Track-1 Step-4 agentic-loop finalize backstop.

use super::apply_loop_finalize;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use serde_json::json;

/// Build an [`EditorState`] whose active page holds `nodes` as its top-level
/// children — the shape the agentic loop assembles by loop end (sections
/// inserted under the active page, the same forest contract the orchestrator's
/// per-subtask passes assume: each top-level node is a section). `InsertSubtree`
/// remaps node ids, so the tests below look nodes up BY NAME, not by the
/// authored id.
fn state_with_forest(nodes: serde_json::Value) -> EditorState {
    let parsed: Vec<jian_ops_schema::node::PenNode> =
        serde_json::from_value(nodes).expect("valid PenNode forest");
    let mut state = EditorState::new();
    state.apply(EditorCommand::InsertSubtree {
        nodes: parsed,
        parent_id: NodeId::NONE,
        page_id: None,
    });
    state
}

/// First node (depth-first) whose name matches `name`.
fn find_by_name<'a>(
    nodes: &'a [jian_ops_schema::node::PenNode],
    name: &str,
) -> Option<&'a jian_ops_schema::node::PenNode> {
    for n in nodes {
        if n.base().name.as_deref() == Some(name) {
            return Some(n);
        }
        if let Some(c) = n.children() {
            if let Some(hit) = find_by_name(c, name) {
                return Some(hit);
            }
        }
    }
    None
}

fn role_of(state: &EditorState, name: &str) -> Option<String> {
    find_by_name(state.active_children(), name).and_then(|n| n.base().role.clone())
}

fn fill_of(state: &EditorState, name: &str) -> Option<serde_json::Value> {
    let n = find_by_name(state.active_children(), name)?;
    let v = serde_json::to_value(n).ok()?;
    Some(v.get("fill").cloned().unwrap_or(json!(null)))
}

/// `apply_loop_finalize` on a small whole-doc forest must run the Class-A
/// sequence: it resolves semantic roles (proving `resolve_forest_roles` ran)
/// AND strips a white structural-wrapper fill (proving `post_pass_forest`'s
/// structural-wrapper-transparency ran on the live document).
#[test]
fn loop_finalize_resolves_roles_and_strips_white_wrapper() {
    // Two top-level sections (the loop's forest). The first is named "Header"
    // with NO role — role inference must assign navbar. The second is an
    // explicitly-`section`-roled wrapper glm gave an inner-card white fill on
    // its CHILD wrapper — the structural-wrapper-transparency pass must strip
    // that inner white wrapper.
    let mut state = state_with_forest(json!([
        {
            "type": "frame",
            "id": "header",
            "name": "Header",
            "x": 0, "y": 0, "width": 1200, "height": 64,
            "children": [
                { "type": "text", "id": "logo", "name": "Logo", "content": "Acme" }
            ]
        },
        {
            "type": "frame",
            "id": "content",
            "name": "Content",
            "x": 0, "y": 64, "width": 1200, "height": 400,
            "children": [
                {
                    "type": "frame",
                    "id": "wrapper",
                    "name": "Inner Section",
                    "role": "section",
                    "x": 0, "y": 0, "width": 1200, "height": 400,
                    "fill": [{ "type": "solid", "color": "#FFFFFF" }],
                    "children": [
                        { "type": "text", "id": "body", "name": "Body", "content": "Hello" }
                    ]
                }
            ]
        }
    ]));

    apply_loop_finalize(&mut state);

    // (a) Roles resolved: the roleless "Header" frame inferred a navbar role.
    assert_eq!(
        role_of(&state, "Header").as_deref(),
        Some("navbar"),
        "resolve_forest_roles must run — the 'Header' frame should infer the navbar role"
    );

    // (b) White structural wrapper stripped: the nested `section`-roled
    // wrapper's white fill is forced transparent.
    assert_eq!(
        fill_of(&state, "Inner Section"),
        Some(json!([])),
        "post_pass_forest structural-wrapper-transparency must run — the white section fill should be stripped"
    );
}

/// When the doc is a single page-root wrapper (sections inside one page frame —
/// the realistic agentic-loop structure), the Class-A section passes run on the
/// wrapper's CHILDREN: roles resolve + a white inner-section wrapper is stripped
/// while the page-root's OWN background fill survives (it is the page surface,
/// not a redundant section fill).
#[test]
fn loop_finalize_preserves_page_root_background() {
    let mut state = state_with_forest(json!([{
        "type": "frame",
        "id": "page",
        "name": "Page",
        "x": 0, "y": 0, "width": 1200, "height": 900,
        "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "header",
                "name": "Header",
                "x": 0, "y": 0, "width": 1200, "height": 64,
                "children": [
                    { "type": "text", "id": "logo", "name": "Logo", "content": "Acme" }
                ]
            },
            {
                "type": "frame",
                "id": "wrapper",
                "name": "Inner Section",
                "role": "section",
                "x": 0, "y": 64, "width": 1200, "height": 400,
                "fill": [{ "type": "solid", "color": "#FFFFFF" }],
                "children": [
                    { "type": "text", "id": "body", "name": "Body", "content": "Hello" }
                ]
            }
        ]
    }]));

    apply_loop_finalize(&mut state);

    // Roles resolved on the page-root's children.
    assert_eq!(
        role_of(&state, "Header").as_deref(),
        Some("navbar"),
        "role inference must run on the page-root's section children"
    );
    // The inner white section wrapper is stripped transparent.
    assert_eq!(
        fill_of(&state, "Inner Section"),
        Some(json!([])),
        "the inner white section wrapper should be stripped"
    );
    // The page-root's OWN background fill survives — it is NOT a section.
    assert_eq!(
        fill_of(&state, "Page"),
        Some(json!([{ "type": "solid", "color": "#FFFFFF" }])),
        "the page-root background fill must survive (it is the page surface, not a section)"
    );
}

/// Empty document → no panic, no-op.
#[test]
fn loop_finalize_on_empty_doc_is_noop() {
    let mut state = EditorState::new();
    apply_loop_finalize(&mut state);
    assert!(state.active_children().is_empty());
}
