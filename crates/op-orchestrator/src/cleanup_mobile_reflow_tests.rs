use crate::{apply_loop_finalize, repair_mobile_trailing_nav_reflow};
use jian_ops_schema::node::{container::ContainerProps, NumberOrExpression, PenNode};
use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use serde_json::{json, Value};

fn state_with_root(root: Value) -> EditorState {
    let root: PenNode = serde_json::from_value(root).expect("valid mobile root");
    let mut state = EditorState::new();
    assert!(state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    }));
    state
}

fn short_content_root() -> Value {
    json!({
        "type": "frame",
        "id": "root",
        "name": "Music App Home",
        "width": 402,
        "height": 1100,
        "layout": "vertical",
        "clipContent": true,
        "children": [
            {
                "type": "frame",
                "id": "status",
                "name": "Status Bar",
                "role": "status-bar",
                "width": "fill_container",
                "height": 62,
                "layout": "none"
            },
            {
                "type": "frame",
                "id": "content",
                "name": "Content Wrapper",
                "width": "fill_container",
                "height": 1038,
                "layout": "vertical",
                "gap": 32,
                "padding": [0, 24, 0, 24],
                "clipContent": true,
                "children": [
                    {"type":"frame", "id":"header", "name":"Header", "width":"fill_container", "height":54},
                    {"type":"frame", "id":"recent", "name":"Recently Played", "width":"fill_container", "height":227},
                    {"type":"frame", "id":"made", "name":"Made For You", "width":"fill_container", "height":222},
                    {"type":"frame", "id":"releases", "name":"New Releases", "width":"fill_container", "height":217},
                    {"type":"frame", "id":"player", "name":"Mini Player", "width":"fill_container", "height":68}
                ]
            },
            {
                "type": "frame",
                "id": "nav",
                "name": "Bottom Tab Bar",
                "role": "bottom-tab-bar",
                "width": 402,
                "height": 72,
                "layout": "horizontal"
            }
        ]
    })
}

fn find_node<'a>(nodes: &'a [PenNode], id: &str) -> Option<&'a PenNode> {
    nodes.iter().find_map(|node| {
        (node.id_str() == id)
            .then_some(node)
            .or_else(|| node.children().and_then(|children| find_node(children, id)))
    })
}

fn find_named<'a>(nodes: &'a [PenNode], name: &str) -> Option<&'a PenNode> {
    nodes.iter().find_map(|node| {
        (node.base().name.as_deref() == Some(name))
            .then_some(node)
            .or_else(|| {
                node.children()
                    .and_then(|children| find_named(children, name))
            })
    })
}

fn find_scene_node<'a>(nodes: &'a [SceneNode], id: &str) -> Option<&'a SceneNode> {
    nodes.iter().find_map(|node| {
        (node.id == id)
            .then_some(node)
            .or_else(|| find_scene_node(&node.children, id))
    })
}

fn frame_props(node: &PenNode) -> &ContainerProps {
    match node {
        PenNode::Frame(frame) => &frame.container,
        other => panic!("expected frame, got {other:?}"),
    }
}

fn numeric_gap(node: &PenNode) -> Option<f64> {
    match frame_props(node).gap.as_ref() {
        Some(NumberOrExpression::Number(value)) => Some(*value),
        _ => None,
    }
}

fn assert_nav_inside_root(state: &EditorState, root_id: &str, nav_id: &str) {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let root = scene
        .active_page()
        .and_then(|page| find_scene_node(&page.children, root_id))
        .expect("resolved root");
    let nav = root
        .children
        .iter()
        .find(|node| node.id == nav_id)
        .expect("direct resolved nav");
    let root_bottom = root.bounds.origin.y + root.bounds.size.y;
    let nav_bottom = nav.bounds.origin.y + nav.bounds.size.y;
    assert!(
        nav_bottom <= root_bottom + 0.5,
        "nav bottom {nav_bottom} must fit root bottom {root_bottom}"
    );
}

#[test]
fn batch_api_hugs_temporary_wrapper_without_shrinking_numeric_root() {
    let mut state = state_with_root(short_content_root());

    assert!(repair_mobile_trailing_nav_reflow(&mut state));

    let root = find_node(state.active_children(), "root").expect("root survives");
    let content = find_node(state.active_children(), "content").expect("content survives");
    assert_eq!(content.height_px(), None, "temporary wrapper must hug");
    assert_eq!(
        numeric_gap(root),
        Some(16.0),
        "mobile chrome regions must retain Pencil-like breathing room"
    );
    assert_eq!(
        root.height_px(),
        Some(1100.0),
        "the numeric root is a minimum height and must not shrink"
    );
    assert_nav_inside_root(&state, "root", "nav");
}

#[test]
fn batch_api_grows_numeric_root_to_long_intrinsic_content() {
    let mut state = state_with_root(json!({
        "type":"frame", "id":"root", "name":"Result", "width":390, "height":844,
        "layout":"vertical", "clipContent":true, "children":[
            {"type":"frame", "id":"status", "name":"Status Bar", "width":"fill_container", "height":62},
            {"type":"frame", "id":"content", "name":"Content Wrapper", "width":"fill_container",
             "height":782, "layout":"vertical", "gap":20, "clipContent":true, "children":[
                {"type":"frame", "id":"a", "name":"A", "width":"fill_container", "height":600},
                {"type":"frame", "id":"b", "name":"B", "width":"fill_container", "height":500}
             ]},
            {"type":"frame", "id":"nav", "name":"Bottom Tab Bar", "role":"bottom-tab-bar",
             "width":390, "height":72, "layout":"horizontal"}
        ]
    }));

    assert!(repair_mobile_trailing_nav_reflow(&mut state));

    let root = find_node(state.active_children(), "root").expect("root survives");
    let content = find_node(state.active_children(), "content").expect("content survives");
    assert_eq!(content.height_px(), None);
    let root_height = root.height_px();
    assert_eq!(
        root_height,
        Some(1286.0),
        "62 + 16 + (600 + 20 + 500) + 16 + 72 must be contained exactly"
    );
    assert_eq!(
        find_node(state.active_children(), "root").and_then(numeric_gap),
        Some(16.0)
    );
    assert_nav_inside_root(&state, "root", "nav");
}

#[test]
fn later_batch_grows_root_after_wrapper_is_already_hugging() {
    let mut state = state_with_root(short_content_root());
    assert!(repair_mobile_trailing_nav_reflow(&mut state));
    assert_eq!(
        find_node(state.active_children(), "root").and_then(PenNodeExt::height_px),
        Some(1100.0)
    );

    let late_section: PenNode = serde_json::from_value(json!({
        "type":"frame", "id":"late", "name":"Late Section",
        "width":"fill_container", "height":600
    }))
    .expect("late section");
    assert!(state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![late_section],
        parent_id: NodeId::new("content".to_string()),
        page_id: None,
    }));

    assert!(
        repair_mobile_trailing_nav_reflow(&mut state),
        "a later batch must remeasure an already-hugging wrapper"
    );
    assert!(find_node(state.active_children(), "root")
        .and_then(PenNodeExt::height_px)
        .is_some_and(|height| height > 1100.0));
    assert_nav_inside_root(&state, "root", "nav");
}

#[test]
fn scene_measurement_uses_active_page_when_root_ids_repeat() {
    let mut inactive_root = short_content_root();
    inactive_root["height"] = json!(3000);
    inactive_root["children"][1]["height"] = json!(2938);
    let document: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "pages": [
            {"id": "inactive", "name": "Inactive", "children": [inactive_root]},
            {"id": "active", "name": "Active", "children": [short_content_root()]}
        ],
        "children": []
    }))
    .expect("two-page document");
    let mut state = EditorState::from_document(document);
    state.ui.active_page_index = 1;

    assert!(repair_mobile_trailing_nav_reflow(&mut state));

    let root = find_node(state.active_children(), "root").expect("active root");
    let content = find_node(state.active_children(), "content").expect("active content");
    assert_eq!(content.height_px(), None);
    assert_eq!(
        root.height_px(),
        Some(1100.0),
        "a same-id root on another page must not drive active-page reflow"
    );
    assert_nav_inside_root(&state, "root", "nav");
}

#[test]
fn batch_api_skips_explicit_numeric_scroll_viewport() {
    let mut state = state_with_root(json!({
        "type":"frame", "id":"root", "name":"Result", "width":390, "height":844,
        "layout":"vertical", "clipContent":true, "children":[
            {"type":"frame", "id":"status", "name":"Status Bar", "width":"fill_container", "height":62},
            {"type":"frame", "id":"viewport", "name":"Scroll Viewport", "role":"viewport",
             "width":"fill_container", "height":782, "layout":"vertical", "clipContent":true,
             "children":[{"type":"frame", "id":"long", "width":"fill_container", "height":1200}]},
            {"type":"frame", "id":"nav", "name":"Bottom Tab Bar", "role":"bottom-tab-bar",
             "width":390, "height":72, "layout":"horizontal"}
        ]
    }));

    assert!(!repair_mobile_trailing_nav_reflow(&mut state));
    assert_eq!(
        find_node(state.active_children(), "viewport").and_then(PenNodeExt::height_px),
        Some(782.0)
    );
    assert_eq!(
        find_node(state.active_children(), "root").and_then(PenNodeExt::height_px),
        Some(844.0)
    );
}

#[test]
fn batch_api_skips_wrapper_that_did_not_consume_the_old_root() {
    let mut value = short_content_root();
    value["children"][1]["height"] = json!(900);
    let mut state = state_with_root(value);

    assert!(!repair_mobile_trailing_nav_reflow(&mut state));
    assert_eq!(
        find_node(state.active_children(), "content").and_then(PenNodeExt::height_px),
        Some(900.0)
    );
    assert_eq!(
        find_node(state.active_children(), "root").and_then(numeric_gap),
        None,
        "a non-candidate root must not receive inferred spacing"
    );
}

#[test]
fn batch_api_preserves_explicit_positive_root_gap() {
    let mut value = short_content_root();
    value["gap"] = json!(12);
    let mut state = state_with_root(value);

    assert!(repair_mobile_trailing_nav_reflow(&mut state));

    let root = find_node(state.active_children(), "root").expect("root survives");
    assert_eq!(numeric_gap(root), Some(12.0));
    assert_nav_inside_root(&state, "root", "nav");
}

#[test]
fn batch_api_preserves_explicit_zero_root_gap() {
    let mut value = short_content_root();
    value["gap"] = json!(0);
    let mut state = state_with_root(value);

    assert!(repair_mobile_trailing_nav_reflow(&mut state));

    let root = find_node(state.active_children(), "root").expect("root survives");
    assert_eq!(
        numeric_gap(root),
        Some(0.0),
        "an authored flush layout must not be reinterpreted as a missing value"
    );
    assert_nav_inside_root(&state, "root", "nav");
}

#[test]
fn batch_api_recognizes_a_role_only_status_bar_for_spacing() {
    let mut value = short_content_root();
    value["children"][0]["id"] = json!("system-chrome");
    value["children"][0]["name"] = json!("System Chrome");
    let mut state = state_with_root(value);

    assert!(repair_mobile_trailing_nav_reflow(&mut state));

    let root = find_node(state.active_children(), "root").expect("root survives");
    assert_eq!(numeric_gap(root), Some(16.0));
    assert_nav_inside_root(&state, "root", "nav");
}

#[test]
fn batch_api_accepts_pencil_style_hug_tab_section() {
    let mut value = short_content_root();
    value["children"][2] = json!({
        "type": "frame",
        "id": "nav",
        "name": "Tab Bar Section",
        "width": "fill_container",
        "layout": "vertical",
        "padding": [12, 21, 21, 21],
        "children": [{
            "type": "frame",
            "id": "tab-pill",
            "name": "Tab Pill",
            "width": "fill_container",
            "justifyContent": "space_around",
            "alignItems": "center",
            "padding": 4,
            "children": [
                {"type":"frame","id":"home","name":"Tab Home - Active","width":"fill_container","layout":"vertical","children":[
                    {"type":"icon_font","id":"home-icon","iconFontName":"house","width":20,"height":20},
                    {"type":"text","id":"home-label","content":"Home","fontSize":11}
                ]},
                {"type":"frame","id":"stats","name":"Tab Stats","width":"fill_container","layout":"vertical","children":[
                    {"type":"icon_font","id":"stats-icon","iconFontName":"chart-bar","width":20,"height":20},
                    {"type":"text","id":"stats-label","content":"Stats","fontSize":11}
                ]},
                {"type":"frame","id":"discover","name":"Tab Discover","width":"fill_container","layout":"vertical","children":[
                    {"type":"icon_font","id":"discover-icon","iconFontName":"compass","width":20,"height":20},
                    {"type":"text","id":"discover-label","content":"Discover","fontSize":11}
                ]},
                {"type":"frame","id":"profile","name":"Tab Profile","width":"fill_container","layout":"vertical","children":[
                    {"type":"icon_font","id":"profile-icon","iconFontName":"user","width":20,"height":20},
                    {"type":"text","id":"profile-label","content":"Profile","fontSize":11}
                ]}
            ]
        }]
    });
    let mut state = state_with_root(value);

    assert!(repair_mobile_trailing_nav_reflow(&mut state));

    let root = find_node(state.active_children(), "root").expect("root survives");
    let content = find_node(state.active_children(), "content").expect("content survives");
    let nav = find_node(state.active_children(), "nav").expect("nav survives");
    let pill = find_node(state.active_children(), "tab-pill").expect("tab pill survives");
    assert_eq!(numeric_gap(root), Some(16.0));
    assert_eq!(content.height_px(), None, "temporary wrapper must hug");
    assert_eq!(nav.height_px(), None, "outer nav section must stay Hug");
    assert_eq!(pill.height_px(), None, "inner tab pill must stay Hug");
    assert_nav_inside_root(&state, "root", "nav");
}

#[test]
fn batch_api_rejects_a_text_only_tab_bar_section() {
    let mut value = short_content_root();
    value["children"][2] = json!({
        "type": "frame",
        "id": "nav",
        "name": "Tab Bar Section",
        "width": "fill_container",
        "layout": "vertical",
        "children": [{
            "type": "frame",
            "id": "tab-pill",
            "name": "Tab Pill",
            "width": "fill_container",
            "justifyContent": "space_around",
            "alignItems": "center",
            "children": [
                {"type":"text","id":"a","content":"Overview"},
                {"type":"text","id":"b","content":"Activity"},
                {"type":"text","id":"c","content":"Settings"}
            ]
        }]
    });
    let mut state = state_with_root(value);

    assert!(!repair_mobile_trailing_nav_reflow(&mut state));
    assert_eq!(
        find_node(state.active_children(), "content").and_then(PenNodeExt::height_px),
        Some(1038.0),
        "a generic tab section is not enough evidence to reinterpret sizing"
    );
    assert_eq!(
        find_node(state.active_children(), "root").and_then(numeric_gap),
        None
    );
}

#[test]
fn batch_api_rejects_unmarked_positioned_prefix_child() {
    let mut value = short_content_root();
    value["children"]
        .as_array_mut()
        .expect("root children")
        .insert(
            2,
            json!({
                "type": "frame",
                "id": "ambiguous-positioned-child",
                "name": "Legacy Positioned Child",
                "x": 24,
                "y": 64,
                "width": 100,
                "height": 100
            }),
        );
    let mut state = state_with_root(value);

    assert!(!repair_mobile_trailing_nav_reflow(&mut state));
    assert_eq!(
        find_node(state.active_children(), "content").and_then(PenNodeExt::height_px),
        Some(1038.0)
    );
    assert_eq!(
        find_node(state.active_children(), "root").and_then(numeric_gap),
        None
    );
}

#[test]
fn batch_api_does_not_infer_gap_for_a_spacer_bearing_root() {
    let mut value = short_content_root();
    value["children"]
        .as_array_mut()
        .expect("root children")
        .insert(
            1,
            json!({
                "type": "frame",
                "id": "authored-spacer",
                "name": "Authored Spacer",
                "width": "fill_container",
                "height": 0
            }),
        );
    let mut state = state_with_root(value);

    assert!(repair_mobile_trailing_nav_reflow(&mut state));

    let root = find_node(state.active_children(), "root").expect("root survives");
    assert_eq!(
        numeric_gap(root),
        None,
        "extra in-flow structure makes the spacing intent ambiguous"
    );
    assert_eq!(
        find_node(state.active_children(), "content").and_then(PenNodeExt::height_px),
        None,
        "containment repair remains available without inferring spacing"
    );
}

#[test]
fn batch_api_root_gap_repair_is_idempotent() {
    let mut state = state_with_root(short_content_root());

    assert!(repair_mobile_trailing_nav_reflow(&mut state));
    assert!(
        !repair_mobile_trailing_nav_reflow(&mut state),
        "the second pass must not rewrite an already-hugging, already-spaced tree"
    );
    assert_eq!(
        find_node(state.active_children(), "root").and_then(numeric_gap),
        Some(16.0)
    );
}

#[test]
fn loop_finalize_runs_trailing_nav_reflow() {
    let mut state = state_with_root(short_content_root());

    apply_loop_finalize(&mut state);

    let root = find_named(state.active_children(), "Music App Home").expect("root survives");
    let content = find_named(state.active_children(), "Content Wrapper").expect("content survives");
    let nav = find_named(state.active_children(), "Bottom Tab Bar").expect("nav survives");
    assert_eq!(content.height_px(), None);
    assert_eq!(
        root.height_px(),
        Some(1100.0),
        "loop finalize must keep the numeric construction height as a minimum"
    );
    assert_nav_inside_root(&state, root.id_str(), nav.id_str());
}

#[test]
fn loop_finalize_does_not_count_absolute_overlay_as_flow_height() {
    let mut root = short_content_root();
    root["children"]
        .as_array_mut()
        .expect("root children")
        .insert(
            2,
            json!({
                "type": "frame",
                "id": "absolute-overlay",
                "name": "Absolute Overlay",
                "role": "overlay",
                "x": 24,
                "y": 64,
                "width": 354,
                "height": 900,
                "layout": "none"
            }),
        );
    let mut state = state_with_root(root);

    apply_loop_finalize(&mut state);

    let root = find_named(state.active_children(), "Music App Home").expect("root survives");
    let content = find_named(state.active_children(), "Content Wrapper").expect("content survives");
    let nav = find_named(state.active_children(), "Bottom Tab Bar").expect("nav survives");
    assert_eq!(
        content.height_px(),
        None,
        "the numeric temporary wrapper must hug on its first repair even when an absolute overlay follows it"
    );
    assert_eq!(
        root.height_px(),
        Some(1100.0),
        "an absolute overlay does not participate in vertical flow"
    );
    assert_nav_inside_root(&state, root.id_str(), nav.id_str());
}
