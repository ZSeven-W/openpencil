//! Document-facing routes that are neither export nor connect: recent-file
//! open (`POST /api/file/open-recent`) and renderer selection push (`POST
//! /api/mcp/selection`), plus the preference-preservation helpers they share.
//! Split out of `web_canvas_server.rs` to keep the spine under the 800-line
//! cap.

use super::*;

pub(super) fn open_recent_file(body: &str, state: &mut WebCanvasState) -> WebReply {
    // Swapping the open document out from under a live session would leave the
    // peers editing a document this daemon no longer has. Refused for every
    // source once a session exists; a no-op when there is none.
    if let Err(refusal) = state.gate_daemon_mutation(
        op_editor_core::CollabGateAction::ReplaceDocument,
        op_editor_core::CollabEditSource::ExternalSync,
    ) {
        return WebReply {
            status: refusal.http_status(),
            body: serde_json::json!({
                "ok": false,
                "error": refusal.code(),
                "message": refusal.to_string(),
            })
            .to_string(),
        };
    }
    let parsed: Option<serde_json::Value> = serde_json::from_str(body).ok();
    let Some(path_s) = parsed
        .as_ref()
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
    else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("Missing path string"),
        };
    };
    if !state
        .editor
        .editor_ui
        .recent_files
        .iter()
        .any(|recent| recent.path == path_s)
    {
        return WebReply {
            status: "404 Not Found",
            body: crate::mcp_serve::rest_error_body("Path is not in recent files"),
        };
    }
    let path = PathBuf::from(&path_s);
    match crate::mcp_serve::load_editor_state(&path) {
        Ok(mut next) => {
            preserve_web_canvas_preferences(&state.editor, &mut next);
            set_file_name_display(&mut next, &path);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            next.editor_ui.touch_recent_file(path_s, now);
            state.editor = next;
            state.current_path = Some(path);
            state.version += 1;
            WebReply {
                status: "200 OK",
                body: crate::mcp_serve::document_sync_ok(state.version),
            }
        }
        Err(e) => {
            let pruned = state.editor.editor_ui.remove_recent_file(&path_s);
            WebReply {
                status: "400 Bad Request",
                body: serde_json::json!({
                    "ok": false,
                    "pruned": pruned,
                    "error": e.to_string(),
                })
                .to_string(),
            }
        }
    }
}

pub(super) fn preserve_web_canvas_preferences(previous: &EditorState, next: &mut EditorState) {
    let previous_selected_model = previous.chat.selected_model_entry().cloned();
    next.editor_ui.theme_mode = previous.editor_ui.theme_mode;
    next.editor_ui.locale = previous.editor_ui.locale;
    next.editor_ui.recent_files = previous.editor_ui.recent_files.clone();
    // Authentication is a daemon/runtime capability, not document state.
    // Managed sync-reset and recent-file loads rebuild EditorState from the
    // document; preserve both the release gate and the display-only profile so
    // they cannot hide the top-bar login/avatar entry after auth initialized.
    next.editor_ui.account_ui_available = previous.editor_ui.account_ui_available;
    next.editor_ui.account = previous.editor_ui.account.clone();
    next.ui_kits = previous.ui_kits.clone();
    next.theme_presets = previous.theme_presets.clone();
    next.theme_presets_dirty = previous.theme_presets_dirty;
    next.editor_ui.agent_settings = previous.editor_ui.agent_settings.clone();
    next.editor_ui.chat_selected_agent = previous.editor_ui.chat_selected_agent;
    next.chat.discovered_models = previous.chat.discovered_models.clone();
    next.rebuild_chat_models();
    if let Some(prev) = previous_selected_model {
        if let Some(idx) = next.chat.available_models.iter().position(|m| {
            m.provider == prev.provider
                && m.value == prev.value
                && m.builtin_provider_id == prev.builtin_provider_id
        }) {
            next.select_chat_model(idx);
        }
    }
}

pub(super) fn set_file_name_display(state: &mut EditorState, path: &std::path::Path) {
    state.editor_ui.file_name_display = path.file_name().map(|n| n.to_string_lossy().into_owned());
}

/// Apply a renderer selection push (`POST /api/mcp/selection`) to the live
/// editor state, mirroring TS `selection.post.ts` + `setSyncSelection`:
/// `selectedIds` must be an array (else 400 with the TS error text); the ids
/// are stored verbatim (TS does no validation — the browser's document is the
/// same synced document, so its ids are normally live here too); a present,
/// non-null `activePageId` switches the active page WHEN the id resolves
/// (documented divergence: TS stores the raw string, Rust keeps a page index
/// so an unknown id is ignored rather than stored). Selection is not part of
/// the document, so no version bump / SSE broadcast happens (TS parity).
pub(super) fn apply_selection_sync(body: &str, state: &mut WebCanvasState) -> WebReply {
    let parsed: Option<serde_json::Value> = serde_json::from_str(body).ok();
    let Some(ids) = parsed
        .as_ref()
        .and_then(|v| v.get("selectedIds"))
        .and_then(|v| v.as_array())
    else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("Missing selectedIds array"),
        };
    };
    let node_ids: Vec<op_editor_core::NodeId> = ids
        .iter()
        .filter_map(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(op_editor_core::NodeId::new)
        .collect();
    let editor = &mut state.editor;
    editor.selection.anchor = node_ids
        .last()
        .cloned()
        .unwrap_or(op_editor_core::NodeId::NONE);
    editor.selection.set = node_ids;
    if let Some(page_id) = parsed
        .as_ref()
        .and_then(|v| v.get("activePageId"))
        .and_then(|v| v.as_str())
    {
        let index = editor
            .doc
            .pages
            .as_ref()
            .and_then(|pages| pages.iter().position(|p| p.id == page_id));
        if let Some(index) = index {
            let _ = editor.set_active_page(index);
        }
    }
    WebReply {
        status: "200 OK",
        body: r#"{"ok":true}"#.to_string(),
    }
}
