//! Document save tool used by the Rust HTTP CLI transport.

use std::collections::BTreeMap;
use std::path::PathBuf;

use op_editor_core::EditorState;

use super::{McpTool, ToolErrorCode, ToolOutcome};

pub struct SaveDocument {
    document_json: String,
}

impl McpTool for SaveDocument {
    fn name(&self) -> &str {
        "save_document"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(path) = args.get("filePath").filter(|path| !path.trim().is_empty()) else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "filePath is required".into(),
            );
        };
        let path = resolve_path(path);
        if let Err(e) = std::fs::write(&path, &self.document_json) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("save document failed: {e}"),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("ok".into(), "true".into());
        out.insert("filePath".into(), path.display().to_string());
        ToolOutcome::Ok(out)
    }
}

pub fn save_document_snapshot(state: &EditorState) -> SaveDocument {
    SaveDocument {
        document_json: serde_json::to_string(&state.doc).unwrap_or_else(|_| "{}".into()),
    }
}

fn resolve_path(raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}
