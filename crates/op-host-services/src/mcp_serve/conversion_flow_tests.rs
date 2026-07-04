//! Scripted code-to-design MCP flow tests.

#![cfg(test)]

use std::path::Path;

use op_editor_core::EditorState;
use serde_json::{json, Value};

use super::*;

fn call(state: &mut EditorState, path: &Path, tool: &str, args: Value) -> Value {
    let line = json!({
        "jsonrpc": "2.0",
        "id": 31,
        "method": "tools/call",
        "params": {
            "name": tool,
            "arguments": args,
        }
    })
    .to_string();
    let response = process_message(state, path, &line)
        .expect("dispatch")
        .expect("response");
    let text = tool_text(&response);
    serde_json::from_str(&text).expect("tool result JSON")
}

fn play(state: &mut EditorState, path: &Path) {
    call(
        state,
        path,
        "upsert_variables",
        json!({
            "key": "tokens:theme.css",
            "variables": {
                "color/primary": {"type": "color", "value": "#3366ff"},
                "space/4": {"type": "number", "value": 16}
            },
            "sourcePath": "src/theme.css",
            "sourceHash": "t1"
        }),
    );
    call(
        state,
        path,
        "upsert_component",
        json!({
            "key": "src/Button.tsx#Button",
            "name": "Button",
            "node_json": {
                "type": "frame",
                "id": "button",
                "name": "Button",
                "children": [
                    {"type": "text", "id": "button-label", "content": "Save"}
                ]
            },
            "sourcePath": "src/Button.tsx",
            "sourceHash": "c1"
        }),
    );
    let status = call(
        state,
        path,
        "conversion_status",
        json!({"kind": "component"}),
    );
    let master_id = status["entries"][0]["nodeId"]
        .as_str()
        .expect("component master id")
        .to_string();
    call(
        state,
        path,
        "upsert_screen",
        json!({
            "key": "route:/",
            "node_json": {
                "type": "frame",
                "id": "home",
                "name": "Home",
                "children": [
                    {"type": "ref", "id": "home-cta", "ref": master_id}
                ]
            },
            "sourcePath": "src/routes/home.tsx",
            "sourceHash": "s1"
        }),
    );
    let lint = call(state, path, "lint_document", json!({}));
    assert!(lint["count"].as_u64().is_some(), "{lint}");
}

#[test]
fn scripted_conversion_is_idempotent() {
    let path = std::env::temp_dir().join(format!(
        "openpencil-conversion-flow-{}.op",
        std::process::id()
    ));
    std::fs::write(&path, r#"{"version":"1","children":[]}"#).expect("seed doc");
    let mut state = load_editor_state(&path).expect("load seed doc");

    play(&mut state, &path);
    let snapshot_1 = serde_json::to_value(&state.doc).unwrap();
    play(&mut state, &path);
    let snapshot_2 = serde_json::to_value(&state.doc).unwrap();
    assert_eq!(
        snapshot_1, snapshot_2,
        "re-running the conversion script must be a no-op"
    );

    let status = call(&mut state, &path, "conversion_status", json!({}));
    assert_eq!(status["total"], 3);
    assert!(status["entries"]
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["status"] == "ok"));
    let reloaded = load_editor_state(&path).expect("reload persisted doc");
    assert_eq!(reloaded.doc.conversion.as_ref().unwrap().entries.len(), 3);
    let _ = std::fs::remove_file(path);
}
