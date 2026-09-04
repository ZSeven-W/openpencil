use super::publish::PersistedFile;
use super::*;
use crate::DesktopApp;
use std::sync::mpsc;
use winit::keyboard::Key;

fn temp_import_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "op-figma-session-{tag}-{}-{nanos}",
        std::process::id()
    ))
}

#[test]
fn cmd_p_preview_cancels_late_figma_import_before_pump() {
    let mut app = DesktopApp::new(None);
    let baseline_doc = app.host.editor_state().doc.clone();
    let baseline_path = app.current_path.clone();

    let import_source = temp_import_dir("preview-cancel");
    let import_output = import_source.with_extension("op");
    let import_fig = import_source.with_extension("fig");
    let (tx, rx) = mpsc::channel();
    let mut imported_state = EditorState::starter();
    imported_state.doc.name = Some("late preview import".to_string());
    tx.send(Ok(CompletedImport {
        prepared: PreparedImport {
            state: imported_state,
            warnings: Vec::new(),
        },
        persisted: Ok(PersistedFile::new(import_output.clone())),
    }))
    .expect("queue late import result");
    app.current_figma_import = Some(FigmaImportSession {
        path: import_fig,
        stage: SessionStage::Converting(rx),
        cancellation: CancellationToken::default(),
        output_mode: ImportOutputMode::CreateFixed,
    });
    {
        let ui = &mut app.host.editor_state_mut().editor_ui;
        ui.import_source = op_editor_core::figma_import_state::ImportSource::Figma;
        ui.figma_import_open = true;
        ui.figma_import_in_progress = true;
    }

    app.zoom_modifier = true;
    app.handle_key_pressed(&Key::Character("p".into()), Some("p"));
    assert!(app.host.preview_active(), "Cmd+P should enter Preview");
    assert_eq!(
        app.host.editor_state().editor_ui.pending_file_action,
        Some(op_editor_core::FileAction::FinishFigmaImport(
            op_editor_core::FigmaImportSelection::Cancel
        ))
    );

    assert!(app.drain_preview_entry_figma_import_cancel());
    assert!(!app.drain_preview_entry_figma_import_cancel());
    assert!(app.current_figma_import.is_none());
    assert!(app
        .host
        .editor_state()
        .editor_ui
        .pending_file_action
        .is_none());

    let outcome = pump(
        &mut app.host,
        &mut app.current_figma_import,
        &mut app.current_path,
        None,
    );

    assert_eq!(outcome, PumpOutcome::Idle);
    assert_eq!(app.host.editor_state().doc, baseline_doc);
    assert_eq!(app.current_path, baseline_path);
}
