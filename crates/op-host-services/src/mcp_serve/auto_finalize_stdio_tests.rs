use super::*;
use op_editor_core::PenNodeExt;

#[test]
fn file_mcp_applier_materializes_empty_image_fill_before_save() {
    let path = std::env::temp_dir().join(format!(
        "openpencil-mcp-image-slot-{}-{}.op",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    std::fs::write(&path, r#"{"version":"1.0.0","children":[]}"#).expect("seed document");
    let mut state = load_editor_state(&path).expect("load document");
    let operations = r##"I(null,{
        "type":"frame",
        "name":"Gallery",
        "width":390,
        "height":844,
        "layout":"vertical",
        "children":[{
            "type":"rectangle",
            "name":"Album Cover",
            "width":240,
            "height":160,
            "fill":[{"type":"image","url":""}]
        }]
    })"##;
    let line = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {"name": "batch_design", "arguments": {"operations": operations}}
    })
    .to_string();

    let response = process_message(&mut state, &path, &line)
        .expect("dispatch")
        .expect("response");
    assert!(!response.contains(r#""isError":true"#), "{response}");

    let saved = load_editor_state(&path).expect("saved document");
    let root = &saved.active_children()[0];
    let slot = root
        .children()
        .expect("gallery children")
        .iter()
        .find(|node| node.base().name.as_deref() == Some("Album Cover"))
        .expect("image slot");
    assert!(
        matches!(slot, jian_ops_schema::node::PenNode::Image(_)),
        "{slot:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn stdio_eof_auto_finalizes_and_saves_the_file_document() {
    let path = std::env::temp_dir().join(format!(
        "openpencil-mcp-stdio-finalize-{}-{}.op",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    std::fs::write(
        &path,
        r##"{
          "version":"1.0.0",
          "children":[{
            "type":"frame", "id":"screen", "name":"Screen",
            "width":390, "height":844, "layout":"vertical",
            "children":[
              {"type":"frame", "id":"status", "name":"Status Bar", "role":"status-bar", "width":"fill_container", "height":62},
              {"type":"frame", "id":"nav", "name":"Bottom Tab Bar", "role":"bottom-tab-bar", "width":"fill_container", "height":72, "layout":"horizontal"},
              {"type":"frame", "id":"content", "name":"Content", "width":"fill_container", "height":200}
            ]
          }]
        }"##,
    )
    .expect("seed document");
    let mut state = load_editor_state(&path).expect("load document");
    let write = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"batch_design","arguments":{"operations":"I(null,{\"type\":\"rectangle\",\"name\":\"Write\",\"x\":500,\"y\":0,\"width\":20,\"height\":20})"}}}"#;
    let mut input = std::io::Cursor::new(format!("{write}\n").into_bytes());
    let mut output = Vec::new();
    let mut auto = auto_finalize::AutoFinalize::for_test(std::time::Duration::ZERO);
    let shutdown = std::sync::atomic::AtomicBool::new(false);
    auto_finalize::run_stdio_session(
        &mut input,
        &mut output,
        &mut state,
        &path,
        &mut auto,
        &shutdown,
    )
    .expect("stdio session");

    let saved = load_editor_state(&path).expect("saved finalized document");
    let root = &saved.active_children()[0];
    let children = root.children().expect("screen children");
    assert_eq!(
        children.last().map(|node| node.id_str()),
        Some("nav"),
        "EOF finalize must anchor the bottom nav as the last child"
    );
    assert!(
        !auto.due(std::time::Instant::now(), state.document_revision()),
        "EOF finalize must record the finalized revision"
    );
    assert!(String::from_utf8_lossy(&output).contains(r#""id":3"#));
    // The reloaded document starts a fresh in-memory revision counter, so the
    // proof that EOF finalize ran is the recorded revision on the session
    // state (plus the structural effect asserted above).
    assert!(
        auto.last_finalized_revision().is_some(),
        "EOF finalize must record the revision it ran against"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn explicit_finalize_design_marks_the_current_revision_finalized() {
    let path = std::env::temp_dir().join(format!(
        "openpencil-mcp-explicit-finalize-{}-{}.op",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    std::fs::write(
        &path,
        r##"{"version":"1.0.0","children":[{
          "type":"frame","id":"root","name":"Landing",
          "width":900,"height":600,"layout":"vertical",
          "children":[{"type":"rectangle","id":"photo","name":"Photo",
            "width":240,"height":160,"fill":[{"type":"image","url":""}]}]
        }]}"##,
    )
    .expect("seed document");
    let mut state = load_editor_state(&path).expect("load document");
    let mut auto = auto_finalize::AutoFinalize::for_test(std::time::Duration::ZERO);
    auto.note_write(std::time::Instant::now());
    let line = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"finalize_design","arguments":{}}}"#;

    let response = process_message_with_auto_finalize(&mut state, &path, line, Some(&mut auto))
        .expect("dispatch")
        .expect("response");
    assert!(!response.contains(r#""isError":true"#), "{response}");
    assert!(
        !auto.due(std::time::Instant::now(), state.document_revision()),
        "explicit finalize must suppress a duplicate idle finalize"
    );
    let _ = std::fs::remove_file(&path);
}
