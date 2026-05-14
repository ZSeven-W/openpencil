//! Stdio MCP server mode for the desktop binary.
//!
//! When `openpencil-desktop --mcp <path>` is invoked, we skip the
//! winit event loop and run a JSON-RPC stdio server against the
//! `.op` file at `<path>`. External CLIs (Claude Code / Codex /
//! Gemini / Copilot) can spawn the binary in this mode to drive
//! the Rust editor exactly the way they drive TS pen-mcp today.
//!
//! Lifecycle per dispatched call:
//!   1. Read a JSON-RPC line from stdin.
//!   2. Re-build the `ToolRegistry` against the current document
//!      so read-tool snapshots reflect the latest state.
//!   3. Dispatch.
//!   4. If the tool emitted an `McpCommand`, the host applier
//!      calls `Document::apply_mcp_command(&cmd)` and saves the
//!      file on success.
//!   5. Write the JSON-RPC response.

use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use openpencil_shell_core::document::Document;
use openpencil_shell_core::mcp::{
    copy_node_snapshot, delete_node_snapshot, document_info_snapshot,
    get_active_theme_snapshot, get_node_snapshot, insert_node_snapshot,
    list_pages_snapshot, list_variables_snapshot, move_node_snapshot,
    replace_node_snapshot, run_stdio_with_applier, selection_snapshot,
    set_active_axis_value_snapshot, set_variable_color_snapshot,
    update_node_snapshot, ToolRegistry,
};

use crate::persistence::{load_from_path, save_to_path};

/// Run the stdio MCP server against `path`. Returns Ok(()) on EOF,
/// Err on unrecoverable IO. The function blocks the calling
/// thread for the lifetime of the stdio connection.
pub fn run(path: PathBuf) -> Result<(), String> {
    let mut doc = Document::empty();
    load_from_path(&mut doc, &path).map_err(|e| format!("load {}: {e}", path.display()))?;

    // Re-built per call so read-tool snapshots reflect the latest
    // state — write commands mutate the doc between dispatches.
    // The registry holds Box<dyn McpTool>, so re-registration is
    // the simplest path until snapshots become incremental.
    let rebuild_registry = |doc: &Document| {
        let mut r = ToolRegistry::default();
        r.register(Box::new(document_info_snapshot(doc)));
        r.register(Box::new(selection_snapshot(doc)));
        r.register(Box::new(get_node_snapshot(doc)));
        r.register(Box::new(list_pages_snapshot(doc)));
        r.register(Box::new(list_variables_snapshot(doc)));
        r.register(Box::new(get_active_theme_snapshot(doc)));
        r.register(Box::new(set_variable_color_snapshot(doc)));
        r.register(Box::new(set_active_axis_value_snapshot(doc)));
        r.register(Box::new(insert_node_snapshot()));
        r.register(Box::new(update_node_snapshot()));
        r.register(Box::new(delete_node_snapshot()));
        r.register(Box::new(move_node_snapshot()));
        r.register(Box::new(copy_node_snapshot()));
        r.register(Box::new(replace_node_snapshot()));
        r
    };

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    // `run_stdio_with_applier` re-uses the same registry for every
    // dispatched line, but each write-command apply mutates `doc`,
    // making the snapshots stale. To keep the contract simple we
    // wrap the loop ourselves: dispatch one call at a time so the
    // registry can be rebuilt against the latest document between
    // calls.
    let mut line = String::new();
    loop {
        line.clear();
        let n = std::io::BufRead::read_line(&mut reader, &mut line)
            .map_err(|e| format!("stdin read: {e}"))?;
        if n == 0 {
            return Ok(()); // EOF
        }
        let registry = rebuild_registry(&doc);
        // Reuse the shell-core dispatcher for parser + apply
        // discipline; feed it a one-line cursor + a closure that
        // mutates the live document and saves on success.
        let mut applier_failed: Option<String> = None;
        let path_for_save = path.clone();
        {
            let doc_ref = &mut doc;
            let path_ref = &path_for_save;
            let applier_failed_ref = &mut applier_failed;
            let mut input = std::io::Cursor::new(line.as_bytes());
            // The shell-core loop reads to EOF; the Cursor over a
            // single line terminates after one iteration so we
            // dispatch + apply + respond exactly once per turn.
            run_stdio_with_applier(&registry, &mut input, &mut writer, |cmd| {
                if !doc_ref.apply_mcp_command(cmd) {
                    return false;
                }
                if let Err(e) = save_to_path(doc_ref, path_ref) {
                    *applier_failed_ref = Some(format!("save failed: {e}"));
                    return false;
                }
                true
            })
            .map_err(|e| format!("dispatch: {e}"))?;
        }
        writer.flush().map_err(|e| format!("stdout flush: {e}"))?;
        if let Some(msg) = applier_failed {
            eprintln!("openpencil-desktop --mcp: {msg}");
        }
    }
}
