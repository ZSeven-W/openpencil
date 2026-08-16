use super::*;
use op_ai::chat_provider::ChatToolExecutor;

#[test]
fn chat_tool_defs_match_ts_crud_subset_and_auth_levels() {
    let defs = chat_tool_defs();
    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "batch_get",
            "snapshot_layout",
            "get_selection",
            "insert_node",
            "update_node",
            "move_node",
            "delete_node",
        ],
        "chat tool set mirrors TS getCrudToolDefs"
    );
    // TS TOOL_AUTH_MAP parity.
    assert_eq!(chat_tool_level("batch_get"), Some("read"));
    assert_eq!(chat_tool_level("insert_node"), Some("create"));
    assert_eq!(chat_tool_level("update_node"), Some("modify"));
    assert_eq!(chat_tool_level("move_node"), Some("modify"));
    assert_eq!(chat_tool_level("delete_node"), Some("delete"));
    // Design-pipeline tools stay excluded from chat v1.
    assert_eq!(chat_tool_level("plan_layout"), None);
    assert_eq!(chat_tool_level("generate_design"), None);
    // Every schema is valid JSON.
    for d in &defs {
        serde_json::from_str::<serde_json::Value>(&d.input_schema_json)
            .unwrap_or_else(|e| panic!("schema for {} unparseable: {e}", d.name));
    }
}

#[test]
fn execute_rejects_tools_outside_the_chat_set() {
    let mut state = EditorState::new();
    let (result, mutated) = execute_chat_tool(&mut state, "delete_page", "{}");
    assert!(result.is_error);
    assert!(!mutated);
    assert!(result.content.contains("not available in chat"));
}

#[test]
fn execute_read_tool_returns_success_envelope() {
    let mut state = EditorState::new();
    let (result, mutated) = execute_chat_tool(&mut state, "get_selection", "{}");
    assert!(!result.is_error, "got {}", result.content);
    assert!(!mutated, "read tools never mutate");
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert_eq!(v["success"], serde_json::Value::Bool(true));
}

#[test]
fn execute_insert_then_update_recolors_node_via_apply_path() {
    // End-to-end-ish: a scripted insert_node creates a rect through
    // the EditorCommand apply path, then update_node recolors it —
    // the GAP #32 acceptance scenario ("make the title red").
    let mut state = EditorState::new();
    let (insert, mutated) = execute_chat_tool(
        &mut state,
        "insert_node",
        r##"{"kind":"rect","name":"Title","x":"10","y":"10","width":"100","height":"40","fill_hex":"#112233"}"##,
    );
    assert!(!insert.is_error, "insert failed: {}", insert.content);
    assert!(mutated, "insert must mutate the document");
    // insert_node's wire result is `{wrote:true}` — the applier
    // allocates the id, so read it back off the live document the
    // way a follow-up batch_get would see it.
    use op_editor_core::PenNodeExt;
    let node_id = state
        .active_children()
        .last()
        .map(|n| n.id_str().to_string())
        .expect("inserted node present on the active page");

    let (update, mutated) = execute_chat_tool(
        &mut state,
        "update_node",
        &format!(r##"{{"nodeId":"{node_id}","fill_hex":"#ff0000"}}"##),
    );
    assert!(!update.is_error, "update failed: {}", update.content);
    assert!(mutated, "update must mutate the document");

    let doc_json = serde_json::to_string(&state.doc).unwrap().to_lowercase();
    assert!(
        doc_json.contains("#ff0000"),
        "node fill must be recolored to #ff0000 via the apply path"
    );
}

#[test]
fn execute_update_unknown_node_reports_tool_error() {
    let mut state = EditorState::new();
    let (result, mutated) = execute_chat_tool(
        &mut state,
        "update_node",
        r##"{"nodeId":"nope","fill_hex":"#ff0000"}"##,
    );
    assert!(result.is_error, "got {}", result.content);
    assert!(!mutated);
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert_eq!(v["success"], serde_json::Value::Bool(false));
}

#[test]
fn apply_modification_replaces_existing_tree_and_backfills_image_src() {
    use op_editor_core::{walkers::find_node, NodeId, PenNodeExt};

    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(
        serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": "n100",
            "name": "Before",
            "x": -100.0,
            "y": 0.0,
            "width": 80.0,
            "height": 80.0,
            "children": []
        }))
        .expect("valid before node"),
    );
    state.active_children_mut().push(
        serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": "n217",
            "name": "Mini Player",
            "x": 0.0,
            "y": 0.0,
            "width": 320.0,
            "height": 180.0,
            "children": [
                {
                    "type": "image",
                    "id": "n218",
                    "name": "Cover Image",
                    "src": "data:image/png;base64,REALIMAGE",
                    "x": 0.0,
                    "y": 0.0,
                    "width": 80.0,
                    "height": 80.0
                },
                {
                    "type": "text",
                    "id": "n220",
                    "name": "Song Title",
                    "content": "Original Title",
                    "x": 90.0,
                    "y": 0.0,
                    "width": 180.0,
                    "height": 24.0
                }
            ]
        }))
        .expect("valid frame node"),
    );
    state.active_children_mut().push(
        serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": "n300",
            "name": "After",
            "x": 400.0,
            "y": 0.0,
            "width": 80.0,
            "height": 80.0,
            "children": []
        }))
        .expect("valid after node"),
    );

    let nodes = vec![(
        "null".to_string(),
        serde_json::json!({
            "type": "frame",
            "id": "n217",
            "name": "Mini Player Rewritten",
            "children": [
                {
                    "type": "image",
                    "id": "n218",
                    "name": "Cover Image Rewritten",
                    "src": "<image>",
                    "width": 10.0,
                    "height": 10.0
                },
                {
                    "type": "text",
                    "id": "n220",
                    "name": "Song Title Rewritten",
                    "content": "B"
                },
                {
                    "type": "frame",
                    "name": "Progress Bar",
                    "width": 220.0,
                    "height": 8.0,
                    "children": []
                }
            ]
        }),
    )];
    let (count, mutated) = apply_design_modification(&mut state, &nodes, &["n217".to_string()]);

    assert_eq!(count, 1);
    assert!(mutated);
    assert_eq!(state.active_children()[0].id_str(), "n100");
    assert_eq!(state.active_children()[1].id_str(), "n217");
    assert_eq!(state.active_children()[2].id_str(), "n300");
    let mini_player = find_node(state.active_children(), &NodeId::new("n217"))
        .expect("existing mini player remains");
    let mini_json = serde_json::to_value(mini_player).expect("mini player serializes");
    assert_eq!(
        mini_json["name"],
        serde_json::json!("Mini Player Rewritten")
    );
    let children = mini_player.children().expect("mini player children");
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].id_str(), "n218");
    assert_eq!(children[1].id_str(), "n220");
    assert_eq!(children[2].base().name.as_deref(), Some("Progress Bar"));

    let image =
        find_node(state.active_children(), &NodeId::new("n218")).expect("cover image remains");
    let image_json = serde_json::to_value(image).expect("image serializes");
    assert_eq!(
        image_json["name"],
        serde_json::json!("Cover Image Rewritten")
    );
    assert_eq!(
        image_json["src"],
        serde_json::json!("data:image/png;base64,REALIMAGE")
    );
    assert_eq!(image_json["width"], serde_json::json!(10.0));

    let title = find_node(state.active_children(), &NodeId::new("n220")).expect("title remains");
    let title_json = serde_json::to_value(title).expect("title serializes");
    assert_eq!(
        title_json["name"],
        serde_json::json!("Song Title Rewritten")
    );
    assert_eq!(title_json["content"], serde_json::json!("B"));

    fn count_id(nodes: &[jian_ops_schema::node::PenNode], id: &str) -> usize {
        nodes
            .iter()
            .map(|node| {
                usize::from(node.id_str() == id)
                    + node.children().map(|kids| count_id(kids, id)).unwrap_or(0)
            })
            .sum()
    }
    assert_eq!(count_id(state.active_children(), "n217"), 1);
    assert_eq!(count_id(state.active_children(), "n218"), 1);
    assert_eq!(count_id(state.active_children(), "n220"), 1);
}

#[test]
fn ui_executor_round_trips_through_the_channel() {
    // Worker side blocks on the ack while the "UI thread" (this
    // test) drains the request and executes against live state —
    // the full pending/apply channel discipline minus winit.
    let (executor, rx) = chat_tool_channel();
    let worker = std::thread::spawn(move || executor.execute("get_selection", "{}"));
    let req = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("worker forwards the tool request");
    assert_eq!(req.name, "get_selection");
    let mut state = EditorState::new();
    let (result, _) = execute_chat_tool(&mut state, &req.name, &req.args_json);
    req.ack.send(result).unwrap();
    let got = worker.join().unwrap();
    assert!(!got.is_error);
    assert!(got.content.contains("\"success\":true"));
}

#[test]
fn ui_executor_reports_abort_when_session_dropped() {
    let (executor, rx) = chat_tool_channel();
    drop(rx); // session went away
    let result = executor.execute("batch_get", "{}");
    assert!(result.is_error);
    assert!(result.content.contains("aborted"));
}
