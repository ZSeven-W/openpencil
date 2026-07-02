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

/// The reported glm bug shape (a single page-root wrapper whose first child is a
/// full-width sidebar) is restructured into a horizontal `[sidebar | Main
/// Content]` app-shell by `apply_loop_finalize`, AND the restructure survives
/// the subsequent Class-A passes — in particular the sidebar is NOT re-widened
/// by `fix_horizontal_overflow` (the guard added for this).
#[test]
fn loop_finalize_restructures_sidebar_dashboard() {
    let mut state = state_with_forest(json!([
        {
            "type": "frame", "id": "root", "name": "Barbershop Client Management",
            "width": 1200, "height": 1775, "layout": "vertical", "gap": 32,
            "children": [
                { "type": "frame", "id": "sb", "name": "Sidebar Navigation",
                  "width": 1200, "height": 605, "layout": "vertical",
                  "children": [
                    { "type": "frame", "id": "logo", "name": "Logo",
                      "width": 1152, "height": 27, "layout": "vertical", "children": [] }
                  ] },
                { "type": "frame", "id": "hd", "name": "Top Header",
                  "width": 1200, "height": 94, "layout": "vertical", "children": [] },
                { "type": "frame", "id": "km", "name": "Key Metrics",
                  "width": 1200, "height": 117, "layout": "horizontal", "children": [] },
                { "type": "frame", "id": "ct", "name": "Client Table Section",
                  "width": 1200, "height": 488, "layout": "vertical", "children": [] },
                { "type": "frame", "id": "ua", "name": "Upcoming Appointments",
                  "width": 1200, "height": 391, "layout": "vertical", "children": [] }
            ]
        }
    ]));
    apply_loop_finalize(&mut state);

    let root = &state.active_children()[0];
    let rv = serde_json::to_value(root).unwrap();
    assert_eq!(
        rv.get("layout").and_then(|l| l.as_str()),
        Some("horizontal"),
        "root restructured to a horizontal app-shell"
    );
    assert_eq!(
        rv["children"].as_array().unwrap().len(),
        2,
        "[sidebar | Main Content]"
    );

    // Sidebar stayed a narrow left column through the whole sequence (the
    // fix_horizontal_overflow guard must NOT re-widen this vertical frame).
    let sidebar = find_by_name(state.active_children(), "Sidebar Navigation").unwrap();
    let sv = serde_json::to_value(sidebar).unwrap();
    assert!(
        sv["width"].as_f64().is_some_and(|w| w <= 300.0),
        "sidebar must stay narrow, got {:?}",
        sv["width"]
    );

    // The data sections moved into the new content column, in order.
    let content =
        find_by_name(state.active_children(), "Main Content").expect("content column created");
    let cv = serde_json::to_value(content).unwrap();
    let names: Vec<String> = cv["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap_or("").to_string())
        .collect();
    assert_eq!(
        names,
        [
            "Top Header",
            "Key Metrics",
            "Client Table Section",
            "Upcoming Appointments"
        ],
        "data sections nested under Main Content in order"
    );
}

#[test]
fn loop_finalize_gives_fill_less_text_a_visible_fill_on_dark_surface() {
    // glm-5.2 in the loop builds a dark design but omits `fill` on text nodes,
    // so they render with the default color — invisible on the dark surfaces it
    // also built. apply_loop_finalize must inject a background-contrasting fill.
    let mut state = state_with_forest(json!([{
        "type": "frame", "id": "root", "name": "Page", "layout": "vertical",
        "width": 1200, "height": 800,
        "fill": [{ "type": "solid", "color": "#0A0A0A" }],
        "children": [{
            "type": "frame", "id": "card", "name": "Card", "layout": "vertical",
            "width": 400, "height": 200,
            "fill": [{ "type": "solid", "color": "#0D0D0D" }],
            "children": [
                { "type": "text", "id": "label", "name": "Label", "content": "Total Clients", "fontSize": 14 }
            ]
        }]
    }]));
    let before = fill_of(&state, "Label").unwrap_or(json!(null));
    assert!(
        before.is_null() || before.as_array().map(|a| a.is_empty()).unwrap_or(false),
        "precondition: Label starts fill-less, got {before:?}"
    );
    apply_loop_finalize(&mut state);
    let after = fill_of(&state, "Label").expect("Label survives finalize");
    assert!(
        after.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "fill-less text on a dark surface must get a visible fill; got {after:?}"
    );
}

#[test]
fn loop_finalize_leaves_text_on_gradient_surface_alone() {
    // A gradient (or image) fill cannot be reduced to a solid hex — guessing a
    // text color there risks dark-on-dark, the mirror of the bug the injection
    // pass fixes. Texts under an indeterminate surface must be left fill-less.
    let mut state = state_with_forest(json!([{
        "type": "frame", "id": "root", "name": "Page", "layout": "vertical",
        "width": 1200, "height": 800,
        "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        "children": [{
            "type": "frame", "id": "hero", "name": "Hero", "layout": "vertical",
            "width": 1200, "height": 400,
            "fill": [{ "type": "linear_gradient",
                       "stops": [ { "offset": 0.0, "color": "#0B1020" },
                                  { "offset": 1.0, "color": "#3B0764" } ] }],
            "children": [
                { "type": "text", "id": "h", "name": "Hero Title", "content": "Launch", "fontSize": 42 }
            ]
        }]
    }]));
    apply_loop_finalize(&mut state);
    let after = fill_of(&state, "Hero Title").unwrap_or(json!(null));
    assert!(
        after.is_null() || after.as_array().map(|a| a.is_empty()).unwrap_or(false),
        "text on a gradient surface must NOT get a guessed fill; got {after:?}"
    );
}

#[test]
fn theme_polarity_split_variables_are_healed() {
    // test07021's verbatim split: a DARK design whose active (Light-slot)
    // values are dark for bg/text but stock-light for surface-2 — the chip
    // resolved #F1F5F9 on a #0A0A0A page. The dark value exists in the other
    // slot; the polarity pass must adopt it. text-primary (correctly light on
    // dark) must be left alone.
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "variables": {
            "color-bg-deep": {"type":"color","value":[
                {"value":"#0A0A0A","theme":{"Mode":"Light"}},
                {"value":"#0F172A","theme":{"Mode":"Dark"}}]},
            "color-surface-2": {"type":"color","value":[
                {"value":"#F1F5F9","theme":{"Mode":"Light"}},
                {"value":"#334155","theme":{"Mode":"Dark"}}]},
            "color-text-primary": {"type":"color","value":[
                {"value":"#FAFAFA","theme":{"Mode":"Light"}},
                {"value":"#F1F5F9","theme":{"Mode":"Dark"}}]}
        },
        "themes": {"Mode": ["Light", "Dark"]},
        "children": [
            {"type":"frame","id":"root","name":"Page","width":1200,"height":800,
             "fill":[{"type":"solid","color":"$color-bg-deep"}],"children":[]}
        ]
    }))
    .expect("valid doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::loop_finalize::fix_theme_variable_polarity(&mut sink);
    let s2 = state
        .resolve_color_variable_hex("color-surface-2")
        .expect("resolves");
    assert!(
        s2.eq_ignore_ascii_case("#334155"),
        "light-slot surface-2 adopted the dark value, got {s2}"
    );
    let tp = state
        .resolve_color_variable_hex("color-text-primary")
        .expect("resolves");
    assert!(
        tp.eq_ignore_ascii_case("#FAFAFA"),
        "correctly-opposite text variable untouched, got {tp}"
    );
}
