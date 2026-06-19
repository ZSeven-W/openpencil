//! Per-call `filePath` routing for the desktop MCP transports.
//!
//! The TypeScript MCP treats `filePath` as the target document for
//! most tools. The Rust desktop transports are still started with a
//! primary document path, but when a tool call supplies a different
//! `filePath`, dispatch against that file for this call only.

use std::path::{Path, PathBuf};

const LIVE_CANVAS_SENTINELS: &[&str] = &["live://canvas", "live://canvas/current"];

struct TargetFileCall {
    tool: String,
    path: PathBuf,
}

pub fn process_message_for_file_path_arg(
    current_path: Option<&Path>,
    line: &str,
) -> Result<Option<String>, String> {
    let Some(target) = target_file_call(line, current_path)? else {
        return Ok(None);
    };
    if target.tool == "open_document" && !target.path.exists() {
        create_empty_document(&target.path)?;
    }
    let mut target_state = super::load_editor_state(&target.path)?;
    let mut applier_failed: Option<String> = None;
    let response = super::process_message_with_applier(&mut target_state, line, |state, cmd| {
        if !state.apply(cmd.clone()) {
            return false;
        }
        if let Err(e) = super::save_editor_state(state, &target.path) {
            applier_failed = Some(format!("save {} failed: {e}", target.path.display()));
            return false;
        }
        true
    })?;
    if let Some(msg) = applier_failed {
        eprintln!("openpencil-desktop mcp: {msg}");
    }
    Ok(response)
}

fn target_file_call(
    line: &str,
    current_path: Option<&Path>,
) -> Result<Option<TargetFileCall>, String> {
    let Some(call) = op_mcp::parse_tool_call(line.trim()) else {
        return Ok(None);
    };
    let source_arg = if call.tool == "save_document" {
        "sourceFilePath"
    } else {
        "filePath"
    };
    let Some(raw) = call
        .arguments
        .get(source_arg)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    if LIVE_CANVAS_SENTINELS.contains(&raw) {
        return Ok(None);
    }
    let target = PathBuf::from(raw);
    if current_path.is_some_and(|current| same_path(&target, current)) {
        return Ok(None);
    }
    Ok(Some(TargetFileCall {
        tool: call.tool,
        path: target,
    }))
}

fn create_empty_document(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    std::fs::write(
        path,
        "{\n  \"version\": \"1.0.0\",\n  \"children\": []\n}\n",
    )
    .map_err(|e| format!("write {}: {e}", path.display()))
}

fn same_path(target: &Path, current: &Path) -> bool {
    match (target.canonicalize(), current.canonicalize()) {
        (Ok(target), Ok(current)) => target == current,
        _ => false,
    }
}
