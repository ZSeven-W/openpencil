//! Ordinary file opens keep the user's mode across the real save/load seam.

use super::*;
use op_editor_core::{EntrySurface, HomeFamily, LeftPanelTab, Tool, WorkspacePhase, WorkspaceView};

struct SavedWork(PathBuf);

impl SavedWork {
    fn new(width: u32, height: u32) -> Self {
        let source = format!(
            r#"{{"version":"1.0.0","children":[
                {{"type":"frame","id":"first","x":0,"y":0,
                  "width":{width},"height":{height},"children":[]}},
                {{"type":"frame","id":"second","x":4000,"y":0,
                  "width":{width},"height":{height},"children":[]}}
            ]}}"#
        );
        let document = jian_ops_schema::load_str(&source).expect("fixture").value;
        let state = EditorState::from_document(document);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "openpencil-open-mode-{}-{nanos}.op",
            std::process::id()
        ));
        save_to_path(&state, &path).expect("save current-schema fixture");
        Self(path)
    }
}

impl Drop for SavedWork {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn ordinary_host(home_visible: bool) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.set_now_ms(500);
    let state = host.editor_state_mut();
    state.editor_ui.entry_surface = EntrySurface::Home;
    state.editor_ui.open_workspace_for_generation(
        HomeFamily::Presentation,
        "previous document brief",
        op_editor_core::TaskDraft::default(),
        42,
        100,
        Some(Tool::Text),
    );
    state.editor_ui.workspace.selected = 7;
    state.editor_ui.home.visible = home_visible;
    host
}

#[test]
fn open_from_home_or_workspace_keeps_normal_mode_and_uses_the_loaded_work() {
    for home_visible in [false, true] {
        for (width, height, family, view) in [
            (
                960,
                540,
                HomeFamily::Presentation,
                WorkspaceView::Single { index: 0 },
            ),
            (375, 812, HomeFamily::AppUi, WorkspaceView::AllBoards),
            (1440, 3000, HomeFamily::Web, WorkspaceView::LongPage),
        ] {
            let work = SavedWork::new(width, height);
            let mut host = ordinary_host(home_visible);
            assert_eq!(
                load_into_host(&mut host, &work.0).unwrap(),
                Some(work.0.clone())
            );
            fit_loaded_document(&mut host, None);

            let state = host.editor_state();
            let ui = &state.editor_ui;
            assert!(!ui.home.visible);
            assert!(host.workspace_visible());
            assert!(ui.workspace.active);
            assert_eq!(ui.workspace.family, family);
            assert_eq!(ui.workspace.view, view);
            assert_eq!(ui.workspace.phase, WorkspacePhase::Done);
            assert_eq!(ui.workspace.selected, 0);
            assert_eq!(ui.workspace.run_epoch, 0);
            assert!(ui.workspace.brief.is_empty());
            assert_eq!(ui.workspace.previous_tool, Some(Tool::Select));
            assert_eq!(state.tool, Tool::Hand);
            assert_eq!(ui.slides_panel.tab, LeftPanelTab::Chat);
            assert!(ui.sidebar_open);
            assert!(!state.is_dirty());
            if family == HomeFamily::Presentation {
                let camera = state.viewport;
                host.apply_workspace_fit(
                    super::super::INITIAL_VIEWPORT_W,
                    super::super::INITIAL_VIEWPORT_H,
                );
                assert_eq!(
                    host.editor_state().viewport,
                    camera,
                    "load fits the selected slide, not the entire deck"
                );
                assert!(
                    camera.zoom > 0.5,
                    "the 4,000 px inter-slide gap must not shrink the opened slide"
                );
            }
        }
    }
}

#[test]
fn open_from_fresh_home_enters_normal_workspace_even_with_a_canvas_startup_preference() {
    let work = SavedWork::new(960, 540);
    let mut host = WidgetHostNative::new();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.entry_surface = EntrySurface::Canvas;
    ui.home.visible = true;
    assert!(!ui.workspace.active);

    load_into_host(&mut host, &work.0).unwrap();
    assert!(host.workspace_visible());
    assert_eq!(
        host.editor_state().editor_ui.entry_surface,
        EntrySurface::Home
    );
}

#[test]
fn open_from_professional_mode_does_not_reenter_a_hidden_normal_workspace() {
    let work = SavedWork::new(960, 540);
    let mut host = ordinary_host(false);
    host.editor_state_mut()
        .editor_ui
        .workspace
        .enter_professional();
    // Professional editing of a Home-created work retains the Home startup
    // preference and an active (but hidden) workspace.
    assert_eq!(
        host.editor_state().editor_ui.entry_surface,
        EntrySurface::Home
    );
    assert!(host.editor_state().editor_ui.workspace.active);

    load_into_host(&mut host, &work.0).unwrap();
    fit_loaded_document(&mut host, None);
    assert!(!host.home_visible());
    assert!(!host.workspace_visible());
    assert!(!host.editor_state().editor_ui.workspace.active);
    assert_eq!(host.editor_state().tool, Tool::Select);
}

#[test]
fn failed_open_preserves_the_normal_workspace_and_document() {
    let work = SavedWork::new(960, 540);
    std::fs::write(&work.0, b"not a document").unwrap();
    let mut host = ordinary_host(false);
    let epoch = host.document_epoch();
    let fingerprint = op_host_services::doc_io::document_fingerprint(host.editor_state());

    assert!(load_into_host(&mut host, &work.0).is_err());
    assert!(host.workspace_visible());
    assert_eq!(host.document_epoch(), epoch);
    assert_eq!(
        op_host_services::doc_io::document_fingerprint(host.editor_state()),
        fingerprint
    );
    assert_eq!(host.editor_state().editor_ui.workspace.run_epoch, 42);
    assert_eq!(
        host.editor_state().editor_ui.workspace.brief,
        "previous document brief"
    );
}
