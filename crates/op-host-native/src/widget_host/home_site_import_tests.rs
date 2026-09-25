//! gl-host tests for Home's one-click website import. Run with
//! `--features gl-host`.

use super::WidgetHostNative;
use op_editor_core::{
    HomeSendMode, HomeSiteImportStatus, NodeId, QualityReport, SiteImportResult, SiteImportSummary,
    WorkspacePhase,
};
use op_editor_ui::widgets::HomeSurface;
use serde_json::json;

fn home_with_link(link: &str) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let home = &mut host.editor_state_mut().editor_ui.home;
    home.visible = true;
    home.site_import.available = true;
    home.set_draft(link);
    host
}

fn imported() -> SiteImportResult {
    let document = serde_json::from_value(json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "html_0", "name": "Acme", "width": 1440.0,
            "height": "fit_content", "layout": "vertical", "children": [
                {"type": "frame", "id": "c1", "name": "Card", "reusable": true,
                 "layout": "vertical", "cornerRadius": 12.0, "children": [
                    {"type": "text", "id": "c1-t", "content": "Fast"}
                 ]},
                {"type": "ref", "id": "c2", "ref": "c1", "name": "Card",
                 "descendants": {"c1-t": {"content": "Safe"}}}
            ]
        }]
    }))
    .unwrap();
    let mut report = QualityReport::default();
    report.ingest_repairs(
        &["structure".to_string()],
        &[op_editor_core::QualityRepairRecord {
            pass: "site-import:components".into(),
            family: "structure".into(),
            node_id: "c1".into(),
            node_name: Some("Card".into()),
            detail: "2 instances".into(),
        }],
        &[],
    );
    report.ingest_audit(&[], Vec::new());
    SiteImportResult {
        document: Box::new(document),
        summary: SiteImportSummary {
            source_url: "https://acme.example/".into(),
            host: "acme.example".into(),
            node_count: 5,
            brand_name: Some("Acme".into()),
            brand_variables: 30,
            colors_bound: 3,
            components: vec![("Card".into(), 2)],
            finalize_fixes: 0,
            warnings: 0,
        },
        report,
    }
}

#[test]
fn a_link_only_draft_offers_import_even_without_a_model() {
    let host = home_with_link("https://acme.example");
    let surface = HomeSurface::for_editor_at(host.editor_state(), 0).unwrap();
    assert_eq!(surface.send_mode(), HomeSendMode::ImportSite);
    assert_eq!(surface.send_label_key(), "home.siteImport.action");

    // A brief that merely mentions a link is still a brief.
    let host = home_with_link("make it like https://acme.example");
    let surface = HomeSurface::for_editor_at(host.editor_state(), 0).unwrap();
    assert_ne!(surface.send_mode(), HomeSendMode::ImportSite);

    // A host that cannot import never offers it.
    let mut host = home_with_link("https://acme.example");
    host.editor_state_mut().editor_ui.home.site_import.available = false;
    let surface = HomeSurface::for_editor_at(host.editor_state(), 0).unwrap();
    assert_ne!(surface.send_mode(), HomeSendMode::ImportSite);
}

#[test]
fn send_queues_one_import_and_the_result_opens_the_workspace() {
    let mut host = home_with_link("acme.example");
    assert!(host.home_send());
    {
        let surface = HomeSurface::for_editor_at(host.editor_state(), 0).unwrap();
        assert_eq!(surface.send_label_key(), "home.siteImport.running");
    }
    let (generation, url) = host.take_home_site_import_request().expect("queued");
    assert_eq!(url, "https://acme.example");
    assert!(host.take_home_site_import_request().is_none());

    assert!(host.finish_home_site_import(generation, Ok(imported())));
    let state = host.editor_state();
    // The imported document is the open document, components rebuilt.
    assert!(
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("c2")).is_some()
    );
    assert_eq!(state.components.components.len(), 1);
    assert!(state.is_dirty(), "an import is unsaved work");
    // The workspace shows it, finished, with the import's report.
    let workspace = &state.editor_ui.workspace;
    assert!(workspace.active && workspace.visible);
    assert_eq!(workspace.phase, WorkspacePhase::Done);
    assert!(workspace.finished_quality().is_some());
    assert!(!state.editor_ui.home.visible);
    assert_eq!(
        state.editor_ui.home.site_import.status,
        HomeSiteImportStatus::Idle
    );
    // The conversation says what was done.
    let last = state.chat.messages.last().expect("transcript");
    assert!(last.content.contains("Card ×2"), "{}", last.content);
    // The replaced starter is handed to the shell (so it drops the old
    // file path) with nothing to rescue.
    let replaced = host.take_replaced_home_document().expect("parked");
    assert!(!replaced.had_unsaved_changes);
}

#[test]
fn a_failed_import_keeps_the_document_and_shows_the_hint() {
    let mut host = home_with_link("https://acme.example");
    let before = serde_json::to_string(&host.editor_state().doc).unwrap();
    assert!(host.home_send());
    let (generation, _) = host.take_home_site_import_request().unwrap();
    assert!(host.finish_home_site_import(generation, Err("HTTP 404".into())));
    let state = host.editor_state();
    assert_eq!(serde_json::to_string(&state.doc).unwrap(), before);
    assert!(state.editor_ui.home.visible);
    let surface = HomeSurface::for_editor_at(state, host.now_ms).unwrap();
    assert_eq!(surface.send_hint_key(), Some("home.siteImport.failed"));
}

#[test]
fn a_stale_result_is_dropped() {
    let mut host = home_with_link("https://acme.example");
    assert!(host.home_send());
    let (generation, _) = host.take_home_site_import_request().unwrap();
    host.editor_state_mut().editor_ui.home.site_import.cancel();
    assert!(!host.finish_home_site_import(generation, Ok(imported())));
    assert!(!host.editor_state().editor_ui.workspace.active);
}
