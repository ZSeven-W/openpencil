use crate::design_agent_tools::execute_design_tool;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use op_editor_ui::layout_scene::SceneNode;
use serde_json::json;

fn find_by_name<'a>(nodes: &'a [PenNode], name: &str) -> Option<&'a PenNode> {
    for node in nodes {
        if node.base().name.as_deref() == Some(name) {
            return Some(node);
        }
        if let Some(found) = node
            .children()
            .and_then(|children| find_by_name(children, name))
        {
            return Some(found);
        }
    }
    None
}

fn find_scene_by_id<'a>(nodes: &'a [SceneNode], id: &str) -> Option<&'a SceneNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find_scene_by_id(&node.children, id) {
            return Some(found);
        }
    }
    None
}

fn temporary_mobile_root() -> serde_json::Value {
    json!({
        "type": "frame", "id":"root-temp", "name": "Music App Home", "width": 402, "height": 1100,
        "layout": "vertical", "clipContent": true, "children": [
            {"type":"frame", "id":"status-temp", "name":"Status Bar", "role":"status-bar",
             "width":"fill_container", "height":62},
            {"type":"frame", "id":"content-temp", "name":"Content Wrapper", "width":"fill_container",
             "height":1038, "layout":"vertical", "gap":32, "children":[
                {"type":"frame", "id":"header-temp", "name":"Header", "width":"fill_container", "height":54},
                {"type":"frame", "id":"section-temp", "name":"Section", "width":"fill_container", "height":400}
             ]},
            {"type":"frame", "id":"nav-temp", "name":"Bottom Tab Bar", "role":"bottom-tab-bar",
             "width":"fill_container", "height":72, "layout":"horizontal"}
        ]
    })
}

fn two_page_state_with_target(
    active_children: serde_json::Value,
    target_children: serde_json::Value,
) -> EditorState {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "pages": [
            {"id": "p1", "name": "Current", "children": active_children},
            {"id": "p2", "name": "Generated", "children": target_children}
        ],
        "children": []
    }))
    .expect("two-page document");
    let mut state = EditorState::from_document(doc);
    state.ui.active_page_index = 0;
    state
}

fn two_page_state(active_children: serde_json::Value) -> EditorState {
    two_page_state_with_target(active_children, json!([]))
}

fn root_with_nested_duplicate_status(id_prefix: &str) -> serde_json::Value {
    json!({
        "type":"frame", "id":format!("{id_prefix}-root"), "name":"Screen",
        "width":390, "height":844, "layout":"vertical", "children":[
            {"type":"frame", "id":format!("{id_prefix}-canonical"),
             "name":"Status Bar", "role":"status-bar", "width":"fill_container",
             "height":62, "children":[
                {"type":"frame", "id":format!("{id_prefix}-levels"), "name":"Levels",
                 "width":80, "height":20}
             ]},
            {"type":"frame", "id":format!("{id_prefix}-header"), "name":"Header",
             "width":"fill_container", "height":100, "children":[
                {"type":"frame", "id":format!("{id_prefix}-duplicate"),
                 "name":"Status Bar Duplicate", "role":"status-bar",
                 "width":"fill_container", "height":44}
             ]}
        ]
    })
}

fn assert_target_content_hugs(state: &EditorState) {
    let pages = state.doc.pages.as_ref().expect("pages survive");
    let content = find_by_name(&pages[1].children, "Content Wrapper").expect("target content");
    assert_eq!(
        serde_json::to_value(content).expect("content json")["height"],
        json!("fit_content"),
        "the explicitly targeted page must receive the same-batch Hug repair"
    );
}

#[test]
fn each_batch_recovers_temporary_mobile_shell_and_grows_root_from_real_content() {
    let mut state = EditorState::new();
    let root_node = json!({
        "type": "frame", "id": "root", "name": "Music App Home",
        "width": 402, "height": 1100, "layout": "vertical", "clipContent": true,
        "children": [
            {"type":"frame", "id":"status", "name":"Status Bar", "role":"status-bar",
             "width":"fill_container", "height":62},
            {"type":"frame", "id":"content", "name":"Content Wrapper",
             "width":"fill_container", "height":1038, "layout":"vertical", "gap":32,
             "clipContent":true, "children":[
                {"type":"frame", "id":"header", "name":"Header", "width":"fill_container", "height":54},
                {"type":"frame", "id":"recent", "name":"Recently Played", "width":"fill_container", "height":227},
                {"type":"frame", "id":"made", "name":"Made For You", "width":"fill_container", "height":222},
                {"type":"frame", "id":"releases", "name":"New Releases", "width":"fill_container", "height":217},
                {"type":"frame", "id":"player", "name":"Mini Player", "width":"fill_container", "height":68}
             ]},
            {"type":"frame", "id":"nav", "name":"Bottom Tab Bar", "role":"bottom-tab-bar",
             "width":"fill_container", "height":72, "layout":"horizontal"}
        ]
    });
    let args = json!({"operations": format!("root=I(null,{root_node})")}).to_string();

    let (first, mutated) = execute_design_tool(&mut state, "batch_design", &args);
    assert!(
        mutated && !first.is_error,
        "first batch failed: {}",
        first.content
    );

    let content = find_by_name(state.active_children(), "Content Wrapper").expect("content");
    assert_eq!(
        serde_json::to_value(content).expect("content json")["height"],
        json!("fit_content"),
        "a filled temporary shell must recover Hug sizing in the same batch"
    );
    let root = find_by_name(state.active_children(), "Music App Home").expect("root");
    assert_eq!(
        root.height_px(),
        Some(1100.0),
        "1100 remains the construction min-height while real content still fits"
    );
    assert_eq!(
        serde_json::to_value(root).expect("root json")["gap"],
        json!(16.0),
        "the same batch must restore the mobile status/content/nav region gap"
    );
    assert!(
        first
            .content
            .contains("preserve the status/content/nav region gap"),
        "the agent must be told about the repaired spacing contract: {}",
        first.content
    );
    let content_id = content.id_str().to_string();
    let status_id = find_by_name(state.active_children(), "Status Bar")
        .expect("status")
        .id_str()
        .to_string();
    let scene = op_pen_loader::editor_state_to_layout_scene(&state);
    let page = scene.active_page().expect("active page");
    let root_scene = find_scene_by_id(&page.children, root.id_str()).expect("root scene");
    let status_scene = find_scene_by_id(&page.children, &status_id).expect("status scene");
    let content_scene = find_scene_by_id(&page.children, &content_id).expect("content scene");
    let nav_id = find_by_name(state.active_children(), "Bottom Tab Bar")
        .expect("nav")
        .id_str()
        .to_string();
    let nav_scene = find_scene_by_id(&page.children, &nav_id).expect("nav scene");
    let status_to_content =
        content_scene.bounds.origin.y - (status_scene.bounds.origin.y + status_scene.bounds.size.y);
    let content_to_nav =
        nav_scene.bounds.origin.y - (content_scene.bounds.origin.y + content_scene.bounds.size.y);
    assert_eq!(status_to_content, 16.0);
    assert_eq!(content_to_nav, 16.0);
    assert!(
        nav_scene.bounds.origin.y + nav_scene.bounds.size.y
            <= root_scene.bounds.origin.y + root_scene.bounds.size.y + 1.0,
        "the trailing nav must be inside the root immediately after the batch"
    );

    let second_args = json!({
        "operations": format!(
            "I(\"{content_id}\",{{\"type\":\"frame\",\"name\":\"More Content\",\"width\":\"fill_container\",\"height\":240}})"
        )
    })
    .to_string();
    let (second, second_mutated) = execute_design_tool(&mut state, "batch_design", &second_args);
    assert!(
        second_mutated && !second.is_error,
        "second batch failed: {}",
        second.content
    );

    let grown = find_by_name(state.active_children(), "Music App Home").expect("grown root");
    assert!(
        grown.height_px().is_some_and(|height| height > 1100.0),
        "the numeric construction root must grow once real Hug content exceeds its min-height"
    );
    let grown_scene = op_pen_loader::editor_state_to_layout_scene(&state);
    let grown_page = grown_scene.active_page().expect("grown page");
    let root_scene = find_scene_by_id(&grown_page.children, grown.id_str()).expect("root");
    let nav_scene = find_scene_by_id(&grown_page.children, &nav_id).expect("nav");
    assert!(
        nav_scene.bounds.origin.y + nav_scene.bounds.size.y
            <= root_scene.bounds.origin.y + root_scene.bounds.size.y + 1.0,
        "the root must keep growing so later batches never push the nav outside"
    );
}

#[test]
fn batch_reflows_the_explicit_non_active_page_without_switching_the_ui() {
    let mut state = two_page_state(json!([
        {"type":"frame", "id":"active-node", "name":"Active Node", "width":100, "height":100}
    ]));
    state.selection.anchor = NodeId::new("active-node");
    state.selection.set = vec![NodeId::new("active-node")];
    let selection_before = state.selection.clone();

    let root_node = temporary_mobile_root();
    let args = json!({
        "pageId": "p2",
        "operations": format!("root=I(null,{root_node})")
    })
    .to_string();

    let (result, mutated) = execute_design_tool(&mut state, "batch_design", &args);
    assert!(
        mutated && !result.is_error,
        "non-active page batch failed: {}",
        result.content
    );
    assert_eq!(
        state.ui.active_page_index, 0,
        "the visible page must not change"
    );
    assert_eq!(
        state.selection, selection_before,
        "target-page reflow must preserve the current page selection"
    );
    let pages = state.doc.pages.as_ref().expect("pages survive");
    assert_eq!(pages[0].children.len(), 1, "current page stays untouched");
    assert_target_content_hugs(&state);
}

#[test]
fn batch_reflows_a_non_active_page_selected_by_numeric_index() {
    let mut state = two_page_state(json!([]));
    let root_node = temporary_mobile_root();
    let args = json!({
        "pageId": "1",
        "operations": format!("root=I(null,{root_node})")
    })
    .to_string();

    let (result, mutated) = execute_design_tool(&mut state, "batch_design", &args);
    assert!(
        mutated && !result.is_error,
        "numeric page batch failed: {}",
        result.content
    );
    assert_eq!(
        state.ui.active_page_index, 0,
        "the visible page must not change"
    );
    assert_target_content_hugs(&state);
}

#[test]
fn batch_reflows_a_non_active_page_selected_by_page_alias() {
    let mut state = two_page_state(json!([]));
    let root_node = temporary_mobile_root();
    let args = json!({
        "page": "p2",
        "operations": format!("root=I(null,{root_node})")
    })
    .to_string();

    let (result, mutated) = execute_design_tool(&mut state, "batch_design", &args);
    assert!(
        mutated && !result.is_error,
        "page alias batch failed: {}",
        result.content
    );
    assert_eq!(
        state.ui.active_page_index, 0,
        "the visible page must not change"
    );
    assert_target_content_hugs(&state);
}

#[test]
fn invalid_explicit_page_selector_is_a_noop_for_the_active_page() {
    let mut state = two_page_state(json!([temporary_mobile_root()]));
    let content_before =
        find_by_name(state.active_children(), "Content Wrapper").expect("active content wrapper");
    assert_eq!(content_before.height_px(), Some(1038.0));
    let active_before = state.ui.active_page_index;
    let selection_before = state.selection.clone();
    let args = json!({
        "pageId": "missing-page",
        "operations": "I(null,{\"type\":\"frame\",\"name\":\"Must Not Land\",\"width\":100,\"height\":100})"
    })
    .to_string();

    let (result, mutated) = execute_design_tool(&mut state, "batch_design", &args);
    assert!(
        !mutated,
        "an invalid page selector must not mutate the document"
    );
    assert!(result.is_error, "an invalid page selector must be rejected");
    assert_eq!(state.ui.active_page_index, active_before);
    assert_eq!(state.selection, selection_before);
    let content_after = find_by_name(state.active_children(), "Content Wrapper")
        .expect("active content wrapper survives");
    assert_eq!(
        content_after.height_px(),
        Some(1038.0),
        "invalid targeting must not fall back to reflowing the active page"
    );
    assert!(
        find_by_name(state.active_children(), "Must Not Land").is_none(),
        "the rejected write must not land on the active page"
    );
}

#[test]
fn target_page_cleanup_and_feedback_do_not_touch_the_current_page() {
    let mut state = two_page_state_with_target(
        json!([root_with_nested_duplicate_status("current")]),
        json!([root_with_nested_duplicate_status("target")]),
    );
    state.selection.anchor = NodeId::new("current-header");
    state.selection.set = vec![NodeId::new("current-header")];
    let selection_before = state.selection.clone();
    let args = json!({
        "pageId": "p2",
        "operations": "I(null,{\"type\":\"frame\",\"name\":\"Target Addition\",\"width\":100,\"height\":100})"
    })
    .to_string();

    let (result, mutated) = execute_design_tool(&mut state, "batch_design", &args);
    assert!(
        mutated && !result.is_error,
        "target-page cleanup batch failed: {}",
        result.content
    );
    assert_eq!(
        state.ui.active_page_index, 0,
        "the visible page must be restored"
    );
    assert_eq!(
        state.selection, selection_before,
        "selection must be restored"
    );

    let pages = state.doc.pages.as_ref().expect("pages survive");
    assert!(
        find_by_name(&pages[0].children, "Status Bar Duplicate").is_some(),
        "the current page's duplicate status bar must not be cleaned by a target-page batch"
    );
    assert!(
        find_by_name(&pages[1].children, "Status Bar Duplicate").is_none(),
        "the target page's duplicate status bar must be cleaned"
    );
    assert!(
        result.content.contains("removed 1 extra status bar"),
        "same-batch feedback must report the target page cleanup: {}",
        result.content
    );
}
