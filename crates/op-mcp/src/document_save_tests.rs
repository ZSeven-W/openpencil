//! Document save tool tests.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::test_fixtures::sample;
use super::{save_document_snapshot, McpTool, ToolErrorCode, ToolOutcome};

#[test]
fn save_document_writes_current_pen_document_to_target_path() {
    let state = sample();
    let dir = temp_dir("save-document");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let out_path = dir.join("copy.op");

    let tool = save_document_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("filePath".into(), out_path.display().to_string());

    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("ok"), Some(&"true".to_string()));
            assert_eq!(out.get("filePath"), Some(&out_path.display().to_string()));
            let saved: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&out_path).expect("saved document"))
                    .expect("saved document json");
            assert_eq!(saved["version"], state.doc.version);
            assert!(saved["children"].is_array() || saved["pages"].is_array());
        }
        other => panic!("expected save ok, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_document_requires_file_path() {
    let tool = save_document_snapshot(&sample());
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, message) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(message.contains("filePath"));
        }
        other => panic!("expected missing argument, got {other:?}"),
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "openpencil-document-save-{label}-{}-{nanos}",
        std::process::id()
    ))
}
