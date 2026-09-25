//! Studio Home's website import on the web host: the send mode, the discard
//! confirm, and landing the daemon's reply. The reply is produced here by
//! hand in the daemon's wire shape — no network.

use super::super::{HomeReplaceIntent, WidgetHost};
use op_editor_core::scene_template_append::template_boards;
use op_editor_core::scene_template_catalog::scene_template_document;
use op_editor_core::{parse_site_import_reply, HomeSendMode, HomeSiteImportStatus, WorkspacePhase};
use op_editor_ui::widgets::HomeSurface;

fn home_with_link(link: &str) -> WidgetHost {
    let mut host = WidgetHost::new();
    let home = &mut host.editor_state.editor_ui.home;
    home.visible = true;
    home.site_import.available = true;
    home.set_draft(link);
    host.last_viewport_w = 1440.0;
    host.last_viewport_h = 900.0;
    host
}

/// What `POST /api/ai/site-import` answers for a one-section page.
fn daemon_reply() -> String {
    serde_json::json!({
        "ok": true,
        "document": {
            "version": "1.0",
            "children": [{
                "type": "frame", "id": "html_0", "name": "Acme", "width": 1440.0,
                "height": "fit_content", "layout": "vertical",
                "children": [{"type": "text", "id": "h1", "content": "Build faster"}]
            }]
        },
        "summary": {
            "sourceUrl": "https://acme.example/?ref=ad",
            "host": "acme.example",
            "nodeCount": 2,
            "brandName": null,
            "components": [],
        },
        "report": op_editor_core::QualityReport::default()
    })
    .to_string()
}

#[test]
fn a_link_offers_import_only_when_the_daemon_serves_it() {
    let host = home_with_link("https://acme.example");
    let surface = HomeSurface::for_editor_at(&host.editor_state, 0).unwrap();
    assert_eq!(surface.send_mode(), HomeSendMode::ImportSite);

    let mut host = home_with_link("https://acme.example");
    host.editor_state.editor_ui.home.site_import.available = false;
    let surface = HomeSurface::for_editor_at(&host.editor_state, 0).unwrap();
    assert_ne!(surface.send_mode(), HomeSendMode::ImportSite);
}

#[test]
fn send_queues_the_import_and_the_reply_opens_the_workspace() {
    let mut host = home_with_link("acme.example");
    assert!(host.home_send());
    let (generation, url) = host.take_home_site_import_request().expect("queued");
    assert_eq!(url, "https://acme.example");
    assert!(host.take_home_site_import_request().is_none());

    let import = parse_site_import_reply(200, &daemon_reply()).expect("reply parses");
    assert!(host.finish_home_site_import(generation, Ok(import)));
    let state = &host.editor_state;
    assert_eq!(state.active_children().len(), 1);
    assert!(state.is_dirty(), "an import is unsaved work");
    assert_eq!(
        state.editor_ui.home.imported_from.as_deref(),
        Some("https://acme.example/"),
        "the origin is kept (sanitized) for the repair policy"
    );
    let workspace = &state.editor_ui.workspace;
    assert!(workspace.active);
    assert_eq!(workspace.phase, WorkspacePhase::Done);
    assert!(!state.editor_ui.home.visible);
    assert_eq!(
        state.editor_ui.home.site_import.status,
        HomeSiteImportStatus::Idle
    );
    let last = state.chat.messages.last().expect("transcript");
    assert!(last.content.contains("acme.example"), "{}", last.content);
    // The page is a new deliverable: Save must not overwrite the daemon's file.
    assert!(host.take_daemon_file_unbind());
}

#[test]
fn unsaved_work_is_confirmed_before_the_import_and_kept_on_failure() {
    let mut host = home_with_link("https://acme.example");
    let source = scene_template_document("slide-deck").expect("embedded on native");
    let boards = template_boards(source, "slide-deck").expect("boards");
    assert!(host.editor_state.adopt_template_boards(boards));
    host.editor_state.mark_document_changed();
    let before = serde_json::to_string(&host.editor_state.doc).unwrap();

    // Dirty page: the press only raises the confirm.
    assert!(!host.home_send());
    assert!(host.take_home_site_import_request().is_none());
    assert_eq!(
        host.take_home_replace_confirm(),
        Some(HomeReplaceIntent::ImportSite)
    );
    // Confirmed: now it is queued — and nothing is swapped yet.
    assert!(host.confirm_home_replace(HomeReplaceIntent::ImportSite));
    let (generation, _) = host.take_home_site_import_request().expect("queued");
    assert_eq!(
        serde_json::to_string(&host.editor_state.doc).unwrap(),
        before
    );

    // A failed import leaves the document exactly as it was.
    let detail =
        parse_site_import_reply(502, r#"{"ok":false,"error":"HTTP 404"}"#).expect_err("a failure");
    assert!(host.finish_home_site_import(generation, Err(detail)));
    assert_eq!(
        serde_json::to_string(&host.editor_state.doc).unwrap(),
        before
    );
    assert!(host.editor_state.editor_ui.home.visible);
    assert!(matches!(
        host.editor_state.editor_ui.home.site_import.status,
        HomeSiteImportStatus::Failed { .. }
    ));
}

#[test]
fn a_stale_reply_is_dropped() {
    let mut host = home_with_link("https://acme.example");
    assert!(host.home_send());
    let (generation, _) = host.take_home_site_import_request().unwrap();
    host.editor_state.editor_ui.home.site_import.cancel();
    let import = parse_site_import_reply(200, &daemon_reply()).unwrap();
    assert!(!host.finish_home_site_import(generation, Ok(import)));
    assert!(!host.editor_state.editor_ui.workspace.active);
}
